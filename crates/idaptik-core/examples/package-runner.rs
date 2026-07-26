use idaptik_core::{load_package, run_package};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: package-runner <idaptik-package.json>");
        return ExitCode::FAILURE;
    };
    let json = match fs::read_to_string(&path) {
        Ok(json) => json,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    match load_package(&json).and_then(run_package) {
        Ok(result) => match serde_json::to_string_pretty(&result) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("cannot serialize result: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("package rejected: {error}");
            ExitCode::FAILURE
        }
    }
}
