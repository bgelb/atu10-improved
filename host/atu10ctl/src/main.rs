use atu10_core::cdc_control::CdcProgrammer;
use atu10_core::device::{flash_image, FakeProgrammer, ProgrammerDevice, F18877_DEVICE_ID};
use atu10_core::hex::HexImage;
use atu10_core::hid::HidProgrammer;
use atu10_core::serial::{serial_smoke, FakeSerialTunnel, SerialTunnel, SystemSerialTunnel};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        print_usage();
        return Ok(());
    };

    match command {
        "probe" => {
            let mut device = open_programmer(&args[1..])?;
            let info = device.as_mut().probe().map_err(|err| err.to_string())?;
            println!(
                "{} protocol={} target={}",
                info.bridge_name, info.protocol_version, info.target_name
            );
        }
        "read-id" => {
            let mut device = open_programmer(&args[1..])?;
            let id = device
                .as_mut()
                .read_target_id()
                .map_err(|err| err.to_string())?;
            println!("target device id: 0x{id:04x}");
        }
        "reset" => {
            let mut device = open_programmer(&args[1..])?;
            device
                .as_mut()
                .reset_target()
                .map_err(|err| err.to_string())?;
            println!("target reset requested");
        }
        "run" => {
            let mut device = open_programmer(&args[1..])?;
            device
                .as_mut()
                .run_target()
                .map_err(|err| err.to_string())?;
            println!("target run requested");
        }
        "flash" => {
            let (backend_args, hex_path) = split_backend_and_path(&args[1..])?;
            let plan = flash_path(backend_args, hex_path)?;
            println!("flashed {} planned rows", plan);
        }
        "verify" => {
            let (backend_args, hex_path) = split_backend_and_path(&args[1..])?;
            let hex_text = fs::read_to_string(hex_path).map_err(|err| err.to_string())?;
            let image = HexImage::parse(&hex_text).map_err(|err| err.to_string())?;
            let plan = atu10_core::flash::FlashPlan::f18877_from_image(&image)
                .map_err(|err| err.to_string())?;
            let mut device = open_programmer(backend_args)?;
            device.probe().map_err(|err| err.to_string())?;
            device.reset_target().map_err(|err| err.to_string())?;
            let target_id = device.read_target_id().map_err(|err| err.to_string())?;
            if target_id != F18877_DEVICE_ID {
                return Err(format!(
                    "unexpected target device id 0x{target_id:04x}; expected PIC16F18877 0x{F18877_DEVICE_ID:04x}"
                ));
            }
            device
                .begin_flash(plan.rows.len() as u32)
                .map_err(|err| err.to_string())?;
            for row in &plan.rows {
                device
                    .verify_range(row.base_word_address, &row.words)
                    .map_err(|err| err.to_string())?;
            }
            device.run_target().map_err(|err| err.to_string())?;
            println!("verified {} planned rows", plan.rows.len());
        }
        "read-words" => {
            let (backend_args, base_word_address, word_count) =
                split_backend_address_count(&args[1..])?;
            let mut device = open_programmer(backend_args)?;
            let words = device
                .read_words(base_word_address, word_count)
                .map_err(|err| err.to_string())?;
            for (line, chunk) in words.chunks(8).enumerate() {
                print!("{:04x}:", base_word_address + (line * 8) as u32);
                for word in chunk {
                    print!(" {word:04x}");
                }
                println!();
            }
        }
        "serial" => {
            let mut serial = open_serial(&args[1..])?;
            serial_smoke(serial.as_mut()).map_err(|err| err.to_string())?;
            println!("serial smoke passed");
        }
        "console" => {
            let (path, baud) = parse_serial_path(&args[1..])?;
            SystemSerialTunnel::console(path, baud).map_err(|err| err.to_string())?;
        }
        "smoke-cycle" => {
            let (programmer_args, serial_args, hex_path) = split_smoke_cycle_args(&args[1..])?;
            let rows =
                if let Some(rows) = try_cdc_smoke_cycle(programmer_args, serial_args, hex_path)? {
                    rows
                } else {
                    let rows = flash_path(programmer_args, hex_path)?;
                    let mut serial = open_serial(serial_args)?;
                    serial_smoke(serial.as_mut()).map_err(|err| err.to_string())?;
                    rows
                };
            println!("smoke-cycle passed after flashing {rows} rows");
        }
        "doctor" => {
            println!("Use `cargo xtask doctor` for full environment checks.");
        }
        "help" | "--help" | "-h" => print_usage(),
        other => return Err(format!("unknown command '{other}'")),
    }

    Ok(())
}

fn flash_path(backend_args: &[String], hex_path: &str) -> Result<usize, String> {
    let hex_text = fs::read_to_string(hex_path).map_err(|err| err.to_string())?;
    let image = HexImage::parse(&hex_text).map_err(|err| err.to_string())?;
    let mut device = open_programmer(backend_args)?;
    let plan = flash_image(device.as_mut(), &image).map_err(|err| err.to_string())?;
    Ok(plan.rows.len())
}

fn try_cdc_smoke_cycle(
    programmer_args: &[String],
    serial_args: &[String],
    hex_path: &str,
) -> Result<Option<usize>, String> {
    let Some((control_path, control_baud)) = parse_control_port_backend(programmer_args)? else {
        return Ok(None);
    };
    let Ok((serial_path, serial_baud)) = parse_serial_path(serial_args) else {
        return Ok(None);
    };
    if control_path != serial_path || control_baud != serial_baud {
        return Ok(None);
    }

    let hex_text = fs::read_to_string(hex_path).map_err(|err| err.to_string())?;
    let image = HexImage::parse(&hex_text).map_err(|err| err.to_string())?;
    let mut device =
        CdcProgrammer::open(control_path, control_baud).map_err(|err| err.to_string())?;
    let plan = flash_image(&mut device, &image).map_err(|err| err.to_string())?;
    serial_smoke(&mut device).map_err(|err| err.to_string())?;
    Ok(Some(plan.rows.len()))
}

fn open_programmer(args: &[String]) -> Result<Box<dyn ProgrammerDevice>, String> {
    if args == ["--fake"] {
        return Ok(Box::<FakeProgrammer>::default());
    }

    if args.len() == 4 && args[0] == "--vid" && args[2] == "--pid" {
        let vid = parse_u16(&args[1])?;
        let pid = parse_u16(&args[3])?;
        return HidProgrammer::open(vid, pid)
            .map(|device| Box::new(device) as Box<dyn ProgrammerDevice>)
            .map_err(|err| err.to_string());
    }

    if let Some((path, baud)) = parse_control_port_backend(args)? {
        return CdcProgrammer::open(path, baud)
            .map(|device| Box::new(device) as Box<dyn ProgrammerDevice>)
            .map_err(|err| err.to_string());
    }

    Err(
        "backend must be `--fake`, `--vid <vid> --pid <pid>`, or `--control-port <path> [--baud <baud>]`"
            .to_string(),
    )
}

fn parse_control_port_backend(args: &[String]) -> Result<Option<(&str, u32)>, String> {
    if args.len() == 2 && args[0] == "--control-port" {
        return Ok(Some((&args[1], 115200)));
    }

    if args.len() == 4 && args[0] == "--control-port" && args[2] == "--baud" {
        let baud = args[3]
            .parse::<u32>()
            .map_err(|err| format!("invalid baud '{}': {err}", args[3]))?;
        return Ok(Some((&args[1], baud)));
    }

    Ok(None)
}

fn open_serial(args: &[String]) -> Result<Box<dyn SerialTunnel>, String> {
    if args == ["--fake"] {
        return Ok(Box::<FakeSerialTunnel>::default());
    }

    let (path, baud) = parse_serial_path(args)?;
    SystemSerialTunnel::open(path, baud)
        .map(|serial| Box::new(serial) as Box<dyn SerialTunnel>)
        .map_err(|err| err.to_string())
}

fn parse_serial_path(args: &[String]) -> Result<(&str, u32), String> {
    if args.len() == 2 && args[0] == "--port" {
        return Ok((&args[1], 115200));
    }

    if args.len() == 4 && args[0] == "--port" && args[2] == "--baud" {
        let baud = args[3]
            .parse::<u32>()
            .map_err(|err| format!("invalid baud '{}': {err}", args[3]))?;
        return Ok((&args[1], baud));
    }

    Err(
        "serial backend must be `--fake`, `--port <path>`, or `--port <path> --baud <baud>`"
            .to_string(),
    )
}

fn split_backend_and_path(args: &[String]) -> Result<(&[String], &str), String> {
    let Some(hex_path) = args.last() else {
        return Err("usage: atu10ctl flash <backend> <firmware.hex>".to_string());
    };
    if args.len() < 2 {
        return Err("usage: atu10ctl flash <backend> <firmware.hex>".to_string());
    }

    Ok((&args[..args.len() - 1], hex_path))
}

fn split_smoke_cycle_args(args: &[String]) -> Result<(&[String], &[String], &str), String> {
    let serial_index = args
        .iter()
        .position(|arg| arg == "--serial")
        .ok_or_else(|| "usage: atu10ctl smoke-cycle <programmer-backend> --serial <serial-backend> <firmware.hex>".to_string())?;
    let Some(hex_path) = args.last() else {
        return Err("usage: atu10ctl smoke-cycle <programmer-backend> --serial <serial-backend> <firmware.hex>".to_string());
    };
    if serial_index == 0 || serial_index + 2 >= args.len() {
        return Err("usage: atu10ctl smoke-cycle <programmer-backend> --serial <serial-backend> <firmware.hex>".to_string());
    }

    Ok((
        &args[..serial_index],
        &args[serial_index + 1..args.len() - 1],
        hex_path,
    ))
}

fn split_backend_address_count(args: &[String]) -> Result<(&[String], u32, usize), String> {
    if args.len() < 3 {
        return Err("usage: atu10ctl read-words <backend> <address> <word-count>".to_string());
    }
    let address = parse_u32(&args[args.len() - 2])?;
    let word_count = parse_usize(&args[args.len() - 1])?;
    Ok((&args[..args.len() - 2], address, word_count))
}

fn parse_u16(value: &str) -> Result<u16, String> {
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u16::from_str_radix(hex, 16).map_err(|err| format!("invalid hex value '{value}': {err}"))
    } else {
        value
            .parse::<u16>()
            .map_err(|err| format!("invalid value '{value}': {err}"))
    }
}

fn parse_u32(value: &str) -> Result<u32, String> {
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u32::from_str_radix(hex, 16).map_err(|err| format!("invalid hex value '{value}': {err}"))
    } else {
        value
            .parse::<u32>()
            .map_err(|err| format!("invalid value '{value}': {err}"))
    }
}

fn parse_usize(value: &str) -> Result<usize, String> {
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        usize::from_str_radix(hex, 16).map_err(|err| format!("invalid hex value '{value}': {err}"))
    } else {
        value
            .parse::<usize>()
            .map_err(|err| format!("invalid value '{value}': {err}"))
    }
}

fn print_usage() {
    println!(
        "usage:\n  atu10ctl probe --fake|--vid <vid> --pid <pid>|--control-port <path> [--baud <baud>]\n  atu10ctl read-id --fake|--vid <vid> --pid <pid>|--control-port <path> [--baud <baud>]\n  atu10ctl reset --fake|--vid <vid> --pid <pid>|--control-port <path> [--baud <baud>]\n  atu10ctl run --fake|--vid <vid> --pid <pid>|--control-port <path> [--baud <baud>]\n  atu10ctl flash --fake|--vid <vid> --pid <pid>|--control-port <path> [--baud <baud>] <firmware.hex>\n  atu10ctl verify --fake|--vid <vid> --pid <pid>|--control-port <path> [--baud <baud>] <firmware.hex>\n  atu10ctl read-words --fake|--vid <vid> --pid <pid>|--control-port <path> [--baud <baud>] <address> <word-count>\n  atu10ctl serial --fake|--port <path> [--baud <baud>]\n  atu10ctl console --port <path> [--baud <baud>]\n  atu10ctl smoke-cycle <programmer-backend> --serial <serial-backend> <firmware.hex>\n  atu10ctl doctor"
    );
}
