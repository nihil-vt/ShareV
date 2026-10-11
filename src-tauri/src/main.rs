// Prevents an additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match sharev_lib::cli::try_run(&args) {
        // A CLI subcommand handled everything; exit with its status.
        Ok(Some(code)) => std::process::exit(code),
        // No subcommand: launch the GUI.
        Ok(None) => sharev_lib::run(),
        Err(err) => {
            eprintln!("sharev: {err}");
            std::process::exit(2);
        }
    }
}
