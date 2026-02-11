use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    io,
    latex::{Arg, LatexBuilder},
};
use anyhow::{Context, Result};
use cooklang::{
    convert::{ConverterBuilder, System, UnitsFile},
    ingredient_list::GroupedIngredient,
    metadata::StdKey,
    Content, Converter, CooklangParser, Extensions, GroupedQuantity, Ingredient, Item, Metadata,
    Quantity, Recipe, Step,
};

#[derive(Debug)]
pub struct RecipeTranspiler<'a> {
    parser: CooklangParser,
    convert_system: Option<System>,
    output_dir: &'a Path,
}

impl<'a> RecipeTranspiler<'a> {
    pub fn new(
        convert_system: Option<System>,
        output_dir: &'a Path,
        units_file: Option<UnitsFile>,
    ) -> Self {
        let converter = if let Some(units_file) = units_file {
            let mut builder = ConverterBuilder::new();
            builder
                .add_bundled_units()
                .expect("Failed to load bundled units");
            builder
                .add_units_file(units_file)
                .expect("Failed to load units file");
            builder.finish().expect("Failed to create converter")
        } else {
            Converter::empty()
        };

        Self {
            parser: CooklangParser::new(Extensions::all(), converter),
            convert_system,
            output_dir,
        }
    }

    pub fn transpile_collection(&self, collection_path: &Path) -> Result<Vec<String>> {
        let files = io::list_dir(collection_path)
            .with_context(|| format!("Failed to read collection: {}", collection_path.display()))?;

        let collection_name = get_collection_name(collection_path)?;
        let mut result_files = Vec::with_capacity(files.len());

        for file in files {
            match self.transpile_recipe(&file, &collection_name) {
                Ok(relative_path) => result_files.push(relative_path),
                Err(e) => {
                    let path = file.display();
                    eprintln!("Warning: Failed to compile recipe {path}: {e}");
                }
            }
        }

        if result_files.is_empty() {
            anyhow::bail!("No recipes were successfully compiled in collection: {collection_name}");
        }

        Ok(result_files)
    }

    fn transpile_recipe(&self, file: &Path, collection_name: &str) -> Result<String> {
        let contents = io::read_file(file)?;
        let file_name = file
            .file_name()
            .context("Invalid file name")?
            .to_str()
            .context("Could not convert to str")?;

        let recipe = self.parse_recipe(&contents, file_name)?;
        let converter = self.parser.converter();

        let mut scaled = recipe;
        if let Some(system) = self.convert_system {
            for error in scaled.convert(system, converter) {
                eprintln!("Warning: {error}");
            }
        }

        let latex = create_recipe(&scaled, converter)?;

        write_recipe(self.output_dir, collection_name, file_name, &latex)
    }

    fn parse_recipe(&self, contents: &str, file_name: &str) -> Result<Recipe> {
        match self.parser.parse(contents).into_result() {
            Ok((recipe, warnings)) => {
                warnings.eprint(file_name, contents, true)?;
                Ok(recipe)
            }
            Err(e) => {
                e.eprint(file_name, contents, true)?;
                Err(e.into())
            }
        }
    }
}

fn get_u64_meta(meta: &Metadata, key: StdKey) -> Option<u64> {
    meta.get(key).and_then(|x| x.as_u64())
}

#[derive(Debug)]
struct RecipeTime {
    prep_time: Option<u64>,
    cook_time: Option<u64>,
}

impl RecipeTime {
    fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            prep_time: get_u64_meta(metadata, StdKey::PrepTime),
            cook_time: get_u64_meta(metadata, StdKey::CookTime),
        }
    }

    fn format_time(minutes: u64) -> String {
        if minutes < 60 {
            format!("{minutes} mins")
        } else {
            let hours = minutes / 60;
            let mins = minutes % 60;
            if mins == 0 {
                format!("{hours} hrs")
            } else {
                format!("{hours} hrs {mins} mins")
            }
        }
    }
}

pub fn create_recipe(recipe: &Recipe, converter: &Converter) -> Result<String> {
    let title = recipe
        .metadata
        .title()
        .context("Recipe must have a title")?;
    let description = recipe
        .metadata
        .description()
        .context("Recipe must have a description")?;

    let mut latex = LatexBuilder::new();
    let recipe_content = build_recipe_content(recipe, converter);

    let meta = recipe_meta(&recipe.metadata);

    Ok(latex
        .add_simple_command("recipeheader", title)
        .add_simple_command("recipedesc", description)
        .add_command("recipemeta", &meta)
        .add_env("recipe", &recipe_content)
        .build())
}

fn build_recipe_content(recipe: &Recipe, converter: &Converter) -> LatexBuilder {
    let mut content = LatexBuilder::new();

    let grouped_ingredients = get_ingredients_by_section(recipe, converter);
    let ingredients = ingredient_list(&grouped_ingredients);
    let instructions = instruction_list(recipe);

    content
        .add_env("ingredients", &ingredients)
        .add_env("instructions", &instructions);

    content
}

fn recipe_meta(meta: &Metadata) -> Vec<Arg> {
    let servings = meta
        .servings()
        .map(|s| s.to_string())
        .expect("Servings must be defined");

    let times = RecipeTime::from_metadata(meta);
    let prep_time = times
        .prep_time
        .map(RecipeTime::format_time)
        .unwrap_or_default();
    let cook_time = times
        .cook_time
        .map(RecipeTime::format_time)
        .unwrap_or_default();

    vec![
        Arg::required(&servings),
        Arg::required(&prep_time),
        Arg::required(&cook_time),
        Arg::required("Moderate"),
    ]
}

fn format_quantity(qty: &Quantity) -> String {
    match qty.unit() {
        Some(unit) => {
            let value = qty.value();
            format!("{value} {unit}")
        }
        None => {
            let value = qty.value();
            format!("{value}")
        }
    }
}

fn sanitize_latex(input: &str) -> String {
    input
        .replace('&', "\\&")
        .replace('%', "\\%")
        .replace('$', "\\$")
        .replace('#', "\\#")
}

fn get_ingredients_by_section<'a>(
    recipe: &'a Recipe,
    converter: &'a Converter,
) -> Vec<(Option<String>, Vec<GroupedIngredient<'a>>)> {
    let mut sections: Vec<(Option<String>, Vec<GroupedIngredient>)> = Vec::new();

    for section in &recipe.sections {
        let mut ingredients: HashMap<String, (&usize, &'a Ingredient, GroupedQuantity)> =
            HashMap::new();

        for content in &section.content {
            if let Content::Step(step) = content {
                for item in &step.items {
                    if let Item::Ingredient { index } = item {
                        let ingredient = &recipe.ingredients[*index];
                        let name = ingredient.name.clone();

                        let grouped_quantity = ingredients.entry(name.clone()).or_insert((
                            index,
                            ingredient,
                            GroupedQuantity::default(),
                        ));

                        if let Some(q) = &ingredient.quantity {
                            grouped_quantity.2.add(q, converter);
                        }
                    }
                }
            }
        }

        let section_name = section.name.clone();
        let mut output_ingredients = ingredients
            .iter()
            .map(|(_name, (index, ingredient, quantity))| GroupedIngredient {
                index: **index,
                ingredient,
                quantity: quantity.clone(),
            })
            .collect::<Vec<_>>();
        output_ingredients.sort_by_key(|gi| gi.index);
        sections.push((section_name.clone(), output_ingredients));
    }

    sections
}

fn ingredient_list(ingredients: &Vec<(Option<String>, Vec<GroupedIngredient>)>) -> LatexBuilder {
    let mut latex = LatexBuilder::new();

    for (section_name, ingredients) in ingredients {
        if let Some(name) = section_name {
            latex.add_simple_command("ingredientsection", &sanitize_latex(name));
        }

        for GroupedIngredient {
            ingredient,
            quantity,
            ..
        } in ingredients
        {
            if !ingredient.modifiers().should_be_listed() {
                continue;
            }

            let mut parts = Vec::new();

            if let Some(qty_str) = quantity
                .iter()
                .map(format_quantity)
                .reduce(|a, b| format!("{a}, {b}"))
            {
                parts.push(qty_str);
            }

            parts.push(ingredient.name.clone());

            let mut args = vec![Arg::required(&sanitize_latex(&parts.join(" ")))];

            if ingredient.modifiers().is_optional() {
                args.push(Arg::optional("\\BooleanTrue"));
            }

            latex.add_command("ingredient", &args);
        }
    }

    latex
}

fn instruction_list(recipe: &Recipe) -> LatexBuilder {
    let mut latex = LatexBuilder::new();

    for section in &recipe.sections {
        if recipe.sections.len() > 1 && section.name.is_some() {
            latex.add_simple_command(
                "instructionsection",
                &sanitize_latex(section.name.as_ref().unwrap()),
            );
        }

        for content in &section.content {
            let instruction = match content {
                Content::Step(step) => step_text(recipe, step),
                Content::Text(text) => text.clone(),
            };

            latex.add_simple_command("step", &sanitize_latex(&instruction));
        }
    }

    latex
}

fn step_text(recipe: &Recipe, step: &Step) -> String {
    step.items
        .iter()
        .map(|item| match item {
            Item::Text { value } => value.clone(),
            Item::Ingredient { index } => recipe.ingredients[*index].display_name().to_string(),
            Item::Cookware { index } => recipe.cookware[*index].name.clone(),
            Item::Timer { index } => format_timer(
                recipe.timers[*index].quantity.as_ref(),
                recipe.timers[*index].name.as_deref(),
            ),
            Item::InlineQuantity { index } => format_quantity(&recipe.inline_quantities[*index]),
        })
        .collect()
}

fn format_timer(quantity: Option<&Quantity>, name: Option<&str>) -> String {
    match (quantity, name) {
        (Some(qty), Some(name)) => format!("{} ({name})", format_quantity(qty)),
        (Some(qty), None) => format_quantity(qty),
        (None, Some(name)) => name.to_string(),
        (None, None) => unreachable!("Timer must have either quantity or name"),
    }
}

pub fn get_collection_name(path: &Path) -> Result<String> {
    path.file_name()
        .context("Invalid collection path")?
        .to_str()
        .context("Invalid collection name")
        .map(String::from)
}

pub fn write_recipe(
    out_dir: &Path,
    collection_name: &str,
    file_name: &str,
    contents: &str,
) -> Result<String> {
    let file_stem = Path::new(file_name)
        .file_stem()
        .context("Invalid recipe file name")?
        .to_str()
        .context("Could not convert to str")?;

    let relative_path = PathBuf::from(collection_name).join(format!("{file_stem}.tex"));

    let target_dir = out_dir.join(collection_name);
    let target_file = out_dir.join(&relative_path);

    io::create_dir_all(&target_dir)?;
    io::write_file(&target_file, contents)?;

    relative_path
        .to_str()
        .context("Failed to compute relative path")
        .map(String::from)
}

pub fn replace_in_main_tex(out_dir: &Path, new_content: &str) -> Result<()> {
    let main_tex = out_dir.join("main.tex");

    let main_tex_contents = io::read_file(&main_tex)?;
    let new_contents = main_tex_contents.replace(r"%{{recipes}}", new_content);

    io::write_file(&main_tex, &new_contents)
}
