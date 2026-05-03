use atu10_core::device::{flash_image, FakeProgrammer, ProgrammerDevice};
use atu10_core::hex::HexImage;
use atu10_core::serial::{serial_smoke, FakeSerialTunnel};
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
            require_fake(&args)?;
            let mut device = FakeProgrammer::default();
            let info = device.probe().map_err(|err| err.to_string())?;
            println!(
                "{} protocol={} target={}",
                info.bridge_name, info.protocol_version, info.target_name
            );
        }
        "reset" => {
            require_fake(&args)?;
            let mut device = FakeProgrammer::default();
            device.reset_target().map_err(|err| err.to_string())?;
            println!("target reset requested");
        }
        "flash" => {
            if args.len() != 3 || args[1] != "--fake" {
                return Err("usage: atu10ctl flash --fake <firmware.hex>".to_string());
            }
            let hex_text = fs::read_to_string(&args[2]).map_err(|err| err.to_string())?;
            let image = HexImage::parse(&hex_text).map_err(|err| err.to_string())?;
            let mut device = FakeProgrammer::default();
            let plan = flash_image(&mut device, &image).map_err(|err| err.to_string())?;
            println!(
                "flashed {} planned rows through fake backend",
                plan.rows.len()
            );
        }
        "serial" => {
            require_fake(&args)?;
            let mut serial = FakeSerialTunnel::default();
            serial_smoke(&mut serial).map_err(|err| err.to_string())?;
            println!("serial smoke passed through fake backend");
        }
        "doctor" => {
            println!("Use `cargo xtask doctor` for full environment checks.");
        }
        "help" | "--help" | "-h" => print_usage(),
        other => return Err(format!("unknown command '{other}'")),
    }

    Ok(())
}

fn require_fake(args: &[String]) -> Result<(), String> {
    if args.len() == 2 && args[1] == "--fake" {
        Ok(())
    } else {
        Err(format!("usage: atu10ctl {} --fake", args[0]))
    }
}

fn print_usage() {
    println!(
        "usage:\n  atu10ctl probe --fake\n  atu10ctl reset --fake\n  atu10ctl flash --fake <firmware.hex>\n  atu10ctl serial --fake\n  atu10ctl doctor"
    );
}
