use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("doctor") => doctor(),
        Some("build-host") => build_host(),
        Some("build-firmware") => build_firmware(),
        Some("test") => test_all(),
        Some("format") => format(args.iter().any(|arg| arg == "--fix")),
        Some("help") | Some("--help") | Some("-h") | None => {
            print_usage();
            Ok(())
        }
        Some(other) => Err(format!("unknown xtask command '{other}'")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn print_usage() {
    println!(
        "usage: cargo xtask <command>\n\ncommands:\n  doctor\n  build-host\n  build-firmware\n  test\n  format [--fix]"
    );
}

fn doctor() -> Result<(), String> {
    let checks = [
        ("cargo", "Rust workspace build/test"),
        ("rustc", "Rust compiler"),
        ("xc8-cc", "Microchip XC8 PIC compiler"),
        ("clang", "host-native C unit tests"),
        ("clang-format", "C formatting"),
        ("mdformat", "Markdown formatting"),
        ("pre-commit", "git hooks"),
    ];

    let mut missing_required = Vec::new();
    for (tool, purpose) in checks {
        let found = command_exists(tool);
        let status = if found { "ok" } else { "missing" };
        println!("{status:7} {tool:12} {purpose}");
        if matches!(tool, "cargo" | "rustc") && !found {
            missing_required.push(tool);
        }
    }

    let dfp_checks = [
        (
            default_dfp_path("PIC12-16F1xxx_DFP", "1.9.258"),
            "PIC16F1454-class bridge DFP",
        ),
        (
            default_dfp_path("PIC16F1xxxx_DFP", "1.31.465"),
            "PIC16F18877-class tuner DFP",
        ),
    ];
    for (path, purpose) in dfp_checks {
        let found = path.is_dir();
        let status = if found { "ok" } else { "missing" };
        println!("{status:7} {:12} {} ({})", "dfp", purpose, path.display());
        if !found {
            missing_required.push("dfp");
        }
    }

    println!();
    println!("Project-managed installs should live under $HOME/.local or $HOME/.atu10-improved.");
    println!("XC8 may need a vendor installer; keep PATH additions local to your shell profile.");

    if missing_required.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "missing required tool(s): {}",
            missing_required.join(", ")
        ))
    }
}

fn build_host() -> Result<(), String> {
    run("cargo", ["build", "-p", "atu10-core", "-p", "atu10ctl"])
}

fn build_firmware() -> Result<(), String> {
    let xc8 = find_first_command(["xc8-cc", "xc8"]).ok_or_else(|| {
        "XC8 not found. Install free MPLAB XC8 and ensure xc8-cc or xc8 is on PATH.".to_string()
    })?;

    run_make(
        Path::new("firmware/programmer-bridge"),
        "build",
        &[("XC8", xc8.as_os_str())],
    )?;
    run_make(
        Path::new("firmware/tuner-controller"),
        "build",
        &[("XC8", xc8.as_os_str())],
    )
}

fn test_all() -> Result<(), String> {
    run("cargo", ["test", "-p", "atu10-core", "-p", "atu10ctl"])?;
    run_c_unity_test(
        "programmer-bridge",
        &[
            "firmware/programmer-bridge/src/protocol_state.c",
            "firmware/programmer-bridge/tests/test_protocol_state.c",
            "firmware/test-support/unity/unity.c",
        ],
    )?;
    run_c_unity_test(
        "tuner-controller",
        &[
            "firmware/tuner-controller/src/uart_app.c",
            "firmware/tuner-controller/tests/test_uart_app.c",
            "firmware/test-support/unity/unity.c",
        ],
    )
}

fn format(fix: bool) -> Result<(), String> {
    if fix {
        run("cargo", ["fmt"])?;
        if command_exists("mdformat") {
            run("mdformat", markdown_files())?;
        }
        if command_exists("clang-format") {
            let files = c_files();
            run("clang-format", chain(["-i".to_string()], files))?;
        }
    } else {
        run("cargo", ["fmt", "--", "--check"])?;
        run(
            "cargo",
            [
                "clippy",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        )?;
        if command_exists("mdformat") {
            run("mdformat", chain(["--check".to_string()], markdown_files()))?;
        } else {
            println!("missing mdformat; skipping Markdown format check");
        }
        if command_exists("clang-format") {
            run(
                "clang-format",
                chain(["--dry-run".to_string(), "--Werror".to_string()], c_files()),
            )?;
        } else {
            println!("missing clang-format; skipping C format check");
        }
    }

    Ok(())
}

fn run_c_unity_test(name: &str, sources: &[&str]) -> Result<(), String> {
    let clang = find_first_command(["clang", "gcc"])
        .ok_or_else(|| "clang or gcc is required for host-native C tests".to_string())?;
    let out_dir = Path::new("target/xtask/c-tests");
    std::fs::create_dir_all(out_dir).map_err(|err| err.to_string())?;
    let binary = out_dir.join(name);

    let mut command = Command::new(clang);
    command
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror"])
        .args(["-Ifirmware/test-support/unity"])
        .args(["-Ifirmware/programmer-bridge/include"])
        .args(["-Ifirmware/tuner-controller/include"])
        .args(sources)
        .arg("-o")
        .arg(&binary);
    run_command(command)?;

    run_path(&binary, std::iter::empty::<&str>())
}

fn run_make(directory: &Path, target: &str, envs: &[(&str, &OsStr)]) -> Result<(), String> {
    let mut command = Command::new("make");
    command.arg(target).current_dir(directory);
    for (key, value) in envs {
        command.env(key, value);
    }
    run_command(command)
}

fn run<I, S>(program: &str, args: I) -> Result<(), String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(program);
    command.args(args);
    run_command(command)
}

fn run_path<I, S>(program: &Path, args: I) -> Result<(), String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(program);
    command.args(args);
    run_command(command)
}

fn run_command(mut command: Command) -> Result<(), String> {
    println!("running: {command:?}");
    let status = command
        .stdin(Stdio::null())
        .status()
        .map_err(|err| err.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed with status {status}: {command:?}"))
    }
}

fn command_exists(program: &str) -> bool {
    find_first_command([program]).is_some()
}

fn find_first_command<I, S>(programs: I) -> Option<PathBuf>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    for program in programs {
        let program = program.as_ref();
        if program.to_string_lossy().contains('/') {
            let path = PathBuf::from(program);
            if path.exists() {
                return Some(path);
            }
            continue;
        }

        for directory in env::var_os("PATH")
            .unwrap_or_default()
            .to_string_lossy()
            .split(':')
        {
            let candidate = Path::new(directory).join(program);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn default_dfp_path(pack: &str, version: &str) -> PathBuf {
    PathBuf::from(env::var_os("HOME").unwrap_or_default())
        .join(".mchp_packs")
        .join("Microchip")
        .join(pack)
        .join(version)
        .join("xc8")
}

fn markdown_files() -> Vec<String> {
    collect_files(["md"])
}

fn c_files() -> Vec<String> {
    collect_files(["c", "h"])
}

fn collect_files<const N: usize>(extensions: [&str; N]) -> Vec<String> {
    let mut files = Vec::new();
    collect_files_from(Path::new("."), &extensions, &mut files);
    files
}

fn collect_files_from<const N: usize>(
    directory: &Path,
    extensions: &[&str; N],
    out: &mut Vec<String>,
) {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or("");
        if matches!(name, ".git" | "target") {
            continue;
        }
        if path.is_dir() {
            collect_files_from(&path, extensions, out);
        } else if let Some(ext) = path.extension().and_then(OsStr::to_str) {
            if extensions.contains(&ext) {
                out.push(path.to_string_lossy().to_string());
            }
        }
    }
}

fn chain<const N: usize>(prefix: [String; N], rest: Vec<String>) -> Vec<String> {
    prefix.into_iter().chain(rest).collect()
}
