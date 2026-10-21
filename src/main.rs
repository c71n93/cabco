//! Compare two committed trees and print their core evaluation.
use std::process::ExitCode;

use cabco::{Classification, Status, git};
use clap::Parser;

#[derive(Parser)]
#[command(name = "cabco")]
enum Arguments {
    Check {
        #[arg(long)]
        base: String,
        #[arg(long)]
        head: String,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(false) => ExitCode::SUCCESS,
        Ok(true) => ExitCode::from(1),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, String> {
    let Arguments::Check { base, head } = Arguments::parse();
    let directory = std::env::current_dir().map_err(|error| error.to_string())?;
    let input = git::collect(&directory, &base, &head)?;
    let result = input.evaluate()?;
    present(&result);
    Ok(result.requires_review())
}

fn present(result: &Classification) {
    if result.paths().next().is_none() {
        println!("AUTONOMOUS — no changed files");
        return;
    }
    for (status, label) in [
        (Status::Review, "REVIEW"),
        (Status::Autonomous, "AUTONOMOUS"),
    ] {
        let paths: Vec<_> = result
            .paths()
            .filter(|path| path.status() == status)
            .collect();
        if paths.is_empty() {
            continue;
        }
        println!("{label} ({})", paths.len());
        for path in paths {
            println!("{}", path.path());
        }
    }
}
