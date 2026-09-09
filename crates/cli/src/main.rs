//! `laser-vector`: command-line front end for the LaserPrep conversion
//! pipeline (`docs/roadmap.md` Phase 7). Depends only on
//! `laserprep_pipeline` and `laserprep_presets` — the same conversion
//! core the desktop app uses (CLAUDE.md Section 5: "O core de
//! conversão deve ser utilizável e testável sem a interface gráfica").
//!
//! Two subcommands:
//! - `convert` — run the pipeline on one image and write an SVG.
//! - `presets` — list the built-in presets and their parameters, so
//!   `--preset` values are always discoverable from the tool itself.

mod args;
mod presets_cmd;

use args::{Cli, Command};
use clap::Parser;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Convert(args) => args::run_convert(args),
        Command::Presets => {
            presets_cmd::run();
            Ok(())
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}
