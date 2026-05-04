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
            let rows = flash_path(programmer_args, hex_path)?;
            let mut serial = open_serial(serial_args)?;
            serial_smoke(serial.as_mut()).map_err(|err| err.to_string())?;
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

    Err(
        "backend must be either `--fake` or `--vid <hex-or-decimal> --pid <hex-or-decimal>`"
            .to_string(),
    )
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

fn print_usage() {
    println!(
        "usage:\n  atu10ctl probe --fake|--vid <vid> --pid <pid>\n  atu10ctl read-id --fake|--vid <vid> --pid <pid>\n  atu10ctl reset --fake|--vid <vid> --pid <pid>\n  atu10ctl run --fake|--vid <vid> --pid <pid>\n  atu10ctl flash --fake|--vid <vid> --pid <pid> <firmware.hex>\n  atu10ctl verify --fake|--vid <vid> --pid <pid> <firmware.hex>\n  atu10ctl serial --fake|--port <path> [--baud <baud>]\n  atu10ctl console --port <path> [--baud <baud>]\n  atu10ctl smoke-cycle <programmer-backend> --serial <serial-backend> <firmware.hex>\n  atu10ctl doctor"
    );
}
