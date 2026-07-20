// NOTE: We use `windows_subsystem = "windows"` so that no console window
// is ever created when launched via file association or double-click.
// In CLI mode, we attach to the parent console so stdout/stderr work normally.
#![windows_subsystem = "windows"]

use std::env;
use std::path::PathBuf;
use std::process::{self, Command};

// ── Console helpers ──────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod console {
    extern "system" {
        fn AttachConsole(dwProcessId: u32) -> i32;
    }

    const ATTACH_PARENT_PROCESS: u32 = u32::MAX; // 0xFFFFFFFF

    /// Attach to the parent process console so CLI output (stdout/stderr) works.
    /// Called early in CLI mode before any println!/eprintln!.
    pub fn attach_to_parent() {
        unsafe {
            // AttachConsole returns 0 on failure — that's fine, it just means
            // the parent has no console (e.g. launched from a GUI file manager).
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod console {
    pub fn attach_to_parent() {}
}

// ── Help ─────────────────────────────────────────────────────────────────────

fn print_help(exe_name: &str) {
    println!("Mermaid Live Editor CLI");
    println!();
    println!("USAGE:");
    println!("  {exe_name} validate <file>    Validate mermaid diagram syntax");
    println!("  {exe_name} check <file>       Alias for validate");
    println!("  {exe_name} -v <file>          Alias for validate");
    println!("  {exe_name} -c <file>          Alias for validate");
    println!("  {exe_name} --help, -h         Show this help");
    println!();
    println!("EXAMPLES:");
    println!("  {exe_name} validate diagram.mmd");
    println!("  {exe_name} check diagram.mmd");
    println!("  {exe_name} -v diagram.mmd");
    println!("  {exe_name} -c diagram.mmd");
    println!("  {exe_name} -h");
    println!();
    println!("DESCRIPTION:");
    println!("  Reads a .mmd or .mermaid file and checks its mermaid syntax.");
    println!("  On success: prints nothing (empty output).");
    println!("  On error: prints the error message.");
    println!();
    println!("  Exit codes:");
    println!("    0 — syntax is valid");
    println!("    1 — syntax error detected");
    println!("    2 — file not found or read error");
    println!("    3 — unknown command");
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn is_mermaid_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".mmd") || lower.ends_with(".mermaid")
}

fn resolve_cli_script() -> Result<PathBuf, String> {
    let exe_path = env::current_exe().map_err(|e| format!("Failed to get exe path: {e}"))?;
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| "Failed to get exe directory".to_string())?;

    let prod_path = exe_dir.join("cli.mjs");
    if prod_path.exists() {
        return Ok(prod_path);
    }

    let mut search_dir = exe_dir.to_path_buf();
    for _ in 0..6 {
        let bundled = search_dir.join("bin").join("cli-bundle.mjs");
        if bundled.exists() {
            return Ok(bundled);
        }
        let dev = search_dir.join("bin").join("cli.mjs");
        if dev.exists() {
            return Ok(dev);
        }
        if let Some(parent) = search_dir.parent() {
            search_dir = parent.to_path_buf();
        } else {
            break;
        }
    }

    Err(format!(
        "Could not find the CLI script. Searched:\n  - {}\n  - (project root)/bin/cli-bundle.mjs\n  - (project root)/bin/cli.mjs",
        prod_path.display()
    ))
}

fn run_node_cli(script: &std::path::Path, args: &[String]) -> ! {
    let output = Command::new("node").arg(script).args(args).output();

    match output {
        Ok(out) => {
            use std::io::{self, Write};
            if !out.stdout.is_empty() {
                let _ = io::stdout().write_all(&out.stdout);
            }
            if !out.stderr.is_empty() {
                let _ = io::stderr().write_all(&out.stderr);
            }
            let code = out.status.code().unwrap_or(1);
            process::exit(code);
        }
        Err(e) => {
            eprintln!("Error: Failed to run Node.js: {e}");
            eprintln!("Make sure Node.js >= 24.13.0 is installed and in your PATH.");
            process::exit(1);
        }
    }
}

// ── CLI argument handling ─────────────────────────────────────────────────────

fn is_validate_command(cmd: &str) -> bool {
    cmd == "validate" || cmd == "check" || cmd == "-v" || cmd == "-c"
}

fn extract_exe_name(args: &[String]) -> &str {
    args[0]
        .split(|c| c == '\\' || c == '/')
        .last()
        .unwrap_or(&args[0])
}

fn handle_cli_args(args: &[String]) {
    // Attach to parent console so println!/eprintln! work when launched from a terminal.
    // Has no effect when launched from a GUI (file association / double-click).
    console::attach_to_parent();

    let first_arg = &args[1];

    // --help / -h
    if first_arg == "--help" || first_arg == "-h" {
        print_help(extract_exe_name(args));
        process::exit(0);
    }

    // validate <file> (aliases: check, -v, -c)
    if is_validate_command(first_arg) {
        match resolve_cli_script() {
            Ok(script) => run_node_cli(&script, &args[1..]),
            Err(err) => {
                eprintln!("Error: {err}");
                process::exit(1);
            }
        }
    }

    // Bare .mmd/.mermaid file path → GUI mode (file association)
    if !is_mermaid_file(first_arg) {
        eprintln!("Error: Unknown command \"{first_arg}\"");
        eprintln!("Run with --help for usage information.");
        process::exit(3);
    }
    // Falls through to GUI for .mmd/.mermaid files
}

// ── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        handle_cli_args(&args);
    }

    mermaid_live_editor_lib::run()
}
