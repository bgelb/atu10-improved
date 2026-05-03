use atu10_core::device::{flash_image, FakeProgrammer, ProgrammerDevice};
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
        "reset" => {
            let mut device = open_programmer(&args[1..])?;
            device
                .as_mut()
                .reset_target()
                .map_err(|err| err.to_string())?;
            println!("target reset requested");
        }
        "flash" => {
            let (backend_args, hex_path) = split_backend_and_path(&args[1..])?;
            let hex_text = fs::read_to_string(hex_path).map_err(|err| err.to_string())?;
            let image = HexImage::parse(&hex_text).map_err(|err| err.to_string())?;
            let mut device = open_programmer(backend_args)?;
            let plan = flash_image(device.as_mut(), &image).map_err(|err| err.to_string())?;
            println!("flashed {} planned rows", plan.rows.len());
        }
        "serial" => {
            let mut serial = open_serial(&args[1..])?;
            serial_smoke(serial.as_mut()).map_err(|err| err.to_string())?;
            println!("serial smoke passed");
        }
        "doctor" => {
            println!("Use `cargo xtask doctor` for full environment checks.");
        }
        "help" | "--help" | "-h" => print_usage(),
        other => return Err(format!("unknown command '{other}'")),
    }

    Ok(())
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

    if args.len() == 2 && args[0] == "--port" {
        return SystemSerialTunnel::open(&args[1], 115200)
            .map(|serial| Box::new(serial) as Box<dyn SerialTunnel>)
            .map_err(|err| err.to_string());
    }

    if args.len() == 4 && args[0] == "--port" && args[2] == "--baud" {
        let baud = args[3]
            .parse::<u32>()
            .map_err(|err| format!("invalid baud '{}': {err}", args[3]))?;
        return SystemSerialTunnel::open(&args[1], baud)
            .map(|serial| Box::new(serial) as Box<dyn SerialTunnel>)
            .map_err(|err| err.to_string());
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
        "usage:\n  atu10ctl probe --fake\n  atu10ctl probe --vid <vid> --pid <pid>\n  atu10ctl reset --fake\n  atu10ctl reset --vid <vid> --pid <pid>\n  atu10ctl flash --fake <firmware.hex>\n  atu10ctl flash --vid <vid> --pid <pid> <firmware.hex>\n  atu10ctl serial --fake\n  atu10ctl serial --port <path> [--baud <baud>]\n  atu10ctl doctor"
    );
}
