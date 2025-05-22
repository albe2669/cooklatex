use clap::Parser;
use cooklang::convert::System;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, help = "The folder containing the LaTeX templates")]
    pub latex_dir: String,

    #[arg(short = 'o', long, help = "The folder to output the LaTeX files to")]
    pub latex_out_dir: String,

    pub collections: Vec<String>,

    /// Convert to a unit system
    #[arg(short, long, alias = "system", value_name = "SYSTEM")]
    pub convert: Option<System>,
}
