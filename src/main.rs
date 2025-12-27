mod cli;
mod io;
mod latex;
mod recipe;

use anyhow::{Context, Result};
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    let latex_dir = &cli.latex_dir;
    let output_dir = &cli.latex_out_dir;

    let units_file = if let Some(units_file) = &cli.units_file {
        let text = std::fs::read_to_string(units_file)
            .with_context(|| format!("Cannot find units file: {}", units_file.display()))?;
        let units = toml::from_str(&text)?;
        Some(units)
    } else {
        None
    };

    io::clone_folder_to_target(latex_dir, output_dir).context("Failed to clone LaTeX directory")?;

    let transpiler = recipe::RecipeTranspiler::new(cli.convert, output_dir, units_file);
    let mut latex = latex::LatexBuilder::new();

    for collection in &cli.collections {
        let collection_path = collection;
        let collection_name = recipe::get_collection_name(collection_path)?;

        latex.add_simple_command("chapter", &collection_name);

        match transpiler.transpile_collection(collection_path) {
            Ok(recipe_files) => {
                let mut iter = recipe_files.iter().peekable();
                while let Some(recipe_file) = iter.next() {
                    latex.add_simple_command("input", recipe_file);
                    if iter.peek().is_some() {
                        latex.add_command("newpage", &[]);
                    }
                }
            }
            Err(e) => eprintln!("Warning: Failed to process collection {collection_name}: {e}"),
        }
    }

    recipe::replace_in_main_tex(output_dir, &latex.build())
        .context("Failed to replace in main.tex")?;

    Ok(())
}
