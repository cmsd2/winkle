mod browser;
mod cli;
mod commands;
mod desktop;
mod entry;
mod fsutil;
mod icons;
mod metadata;
mod paths;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Install(args) => commands::install::run(args),
        Command::List(args) => commands::list::run(args),
        Command::Remove(args) => commands::remove::run(args),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
