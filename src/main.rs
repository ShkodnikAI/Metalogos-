// ── METALOGOS — M1 binary entry point ──────────────────────────────────
// `mlog run <file.mlog>`  — execute a .mlog program

use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mlog", about = "METALOGOS — AI-native programming language")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Run a .mlog program
    Run {
        /// Path to .mlog source file
        file: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file } => {
            let source = match fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: cannot read {:?}: {}", file, e);
                    std::process::exit(1);
                }
            };

            match metalogos::run_program(&source) {
                Ok(output) => {
                    if let Some(result) = output {
                        println!("{}", result);
                    }
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
