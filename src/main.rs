mod cli;
mod error;
mod fetch;
mod markdown;
mod search;

use std::process::ExitCode;

use crate::{cli::Command, error::AppError};

fn main() -> ExitCode {
    match run() {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<String, AppError> {
    match cli::parse(std::env::args().skip(1))? {
        Command::Search(options) => {
            let result = search::search(&options)?;
            serde_json::to_string(&result).map_err(AppError::from)
        }
        Command::Fetch(options) => {
            let result = fetch::fetch(&options.url)?;
            if options.md {
                Ok(result.content)
            } else {
                serde_json::to_string(&result).map_err(AppError::from)
            }
        }
        Command::Help => Ok(cli::usage().to_string()),
    }
}
