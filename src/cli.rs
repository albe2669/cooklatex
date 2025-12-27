use std::path::PathBuf;

use clap::Parser;
use cooklang::convert::System;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, help = "The folder containing the LaTeX templates")]
    pub latex_dir: PathBuf,

    #[arg(short = 'o', long, help = "The folder to output the LaTeX files to")]
    pub latex_out_dir: PathBuf,

    pub collections: Vec<PathBuf>,

    /// Convert to a unit system
    #[arg(short, long, alias = "system", value_name = "SYSTEM")]
    pub convert: Option<System>,

    #[arg(short = 'u', long, help = "Path to a custom units file in TOML format")]
    pub units_file: Option<PathBuf>,
}
