//! `mur` — Multipart UR QR code generator CLI.

#[doc(hidden)]
mod cmd;
#[doc(hidden)]
mod exec;
#[doc(hidden)]
mod styles;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::exec::Exec;

/// Multipart UR QR code generator.
#[derive(Debug, Parser)]
#[command(author, version)]
#[command(propagate_version = true)]
#[command(styles=styles::get_styles())]
#[doc(hidden)]
struct Cli {
    #[command(subcommand)]
    command: MainCommands,
}

#[derive(Debug, Subcommand)]
#[doc(hidden)]
enum MainCommands {
    /// Render a single-frame QR code.
    Single(cmd::single::CommandArgs),
    /// Generate an animated multipart QR sequence.
    Animate(cmd::animate_cmd::CommandArgs),
    /// Dump multipart QR frames as numbered PNGs.
    Frames(cmd::frames::CommandArgs),
}

#[doc(hidden)]
fn main() -> Result<()> {
    let cli = Cli::parse();

    let output = match cli.command {
        MainCommands::Single(args) => args.exec(),
        MainCommands::Animate(args) => args.exec(),
        MainCommands::Frames(args) => args.exec(),
    };
    let output = output?;
    if !output.is_empty() {
        println!("{output}");
    }
    Ok(())
}
