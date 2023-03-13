use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
pub struct Args {
    /// Mode to run in
    #[arg(short, long, value_enum)]
    pub mode: Mode,

    /// Enable debugging
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,

    /// The path to nixpkgs
    pub path: PathBuf,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Mode {
    // Can be removed once the migration is done
    Migrate,
    Warn,
    Error,
}
