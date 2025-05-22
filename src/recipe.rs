use std::path::{Path, PathBuf};

use crate::{io, latex::LatexBuilder};
use anyhow::{Context, Result};
use cooklang::{
    convert::System, ingredient_list::GroupedIngredient, metadata::StdKey, scale::Servings,
    Content, Converter, CooklangParser, Item, Metadata, Quantity, Recipe, ScalableValue,
    ScaledRecipe, Step,
};

#[derive(Debug)]
pub struct RecipeTranspiler<'a> {
    parser: CooklangParser,
    convert_system: Option<System>,
    output_dir: &'a Path,
}

impl<'a> RecipeTranspiler<'a> {
    pub fn new(convert_system: Option<System>, output_dir: &'a Path) -> Self {
        Self {
            parser: CooklangParser::extended(),
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
                Err(e) => eprintln!(
                    "Warning: Failed to compile recipe {}: {}",
                    file.display(),
                    e
                ),
            }
        }

        if result_files.is_empty() {
            anyhow::bail!(
                "No recipes were successfully compiled in collection: {}",
                collection_name
            );
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

        let mut scaled = recipe.default_scale();
        if let Some(system) = self.convert_system {
            for error in scaled.convert(system, converter) {
                eprintln!("Warning: {}", error);
            }
        }

        let latex = create_recipe(scaled, converter)?;

        write_recipe(self.output_dir, collection_name, file_name, &latex)
    }

    fn parse_recipe(
        &self,
        contents: &str,
        file_name: &str,
    ) -> Result<Recipe<Servings, ScalableValue>> {
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
            format!("{} mins", minutes)
        } else {
            let hours = minutes / 60;
            let mins = minutes % 60;
            if mins == 0 {
                format!("{} hrs", hours)
            } else {
                format!("{} hrs {} mins", hours, mins)
            }
        }
    }
}

pub fn create_recipe(recipe: ScaledRecipe, converter: &Converter) -> Result<String> {
    let title = recipe
        .metadata
        .title()
        .context("Recipe must have a title")?;
    let description = recipe
        .metadata
        .description()
        .context("Recipe must have a description")?;

    let mut latex = LatexBuilder::new();
    let recipe_content = build_recipe_content(&recipe, converter)?;

    let meta = recipe_meta(&recipe.metadata);
    let meta = meta.iter().map(|x| x.as_str()).collect::<Vec<&str>>();

    Ok(latex
        .add_simple_command("recipeheader", title)
        .add_simple_command("recipedesc", description)
        .add_command("recipemeta", &meta)
        .add_env("recipe", &recipe_content)
        .build())
}

fn build_recipe_content(recipe: &ScaledRecipe, converter: &Converter) -> Result<LatexBuilder> {
    let mut content = LatexBuilder::new();

    let ingredients = ingredient_list(&recipe.group_ingredients(converter));
    let instructions = instruction_list(recipe);

    content
        .add_env("ingredients", &ingredients)
        .add_env("instructions", &instructions);

    Ok(content)
}

fn recipe_meta(meta: &Metadata) -> Vec<String> {
    let servings = meta
        .servings()
        .and_then(|x| x.first().map(|x| x.to_string()))
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

    vec![servings, prep_time, cook_time, "Moderate".to_string()]
}

fn format_quantity(qty: &Quantity) -> String {
    match qty.unit() {
        Some(unit) => format!("{} {}", qty.value(), unit),
        None => format!("{}", qty.value()),
    }
}

fn ingredient_list(ingredients: &Vec<GroupedIngredient>) -> LatexBuilder {
    let mut latex = LatexBuilder::new();

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

        if ingredient.modifiers().is_optional() {
            parts.push("\\textit{(optional)}".to_string());
        }

        if let Some(qty_str) = quantity
            .iter()
            .map(format_quantity)
            .reduce(|a, b| format!("{}, {}", a, b))
        {
            parts.push(qty_str);
        }

        parts.push(ingredient.display_name().to_string());
        latex.add_simple_command("item", &parts.join(" "));
    }

    latex
}

fn instruction_list(recipe: &ScaledRecipe) -> LatexBuilder {
    let mut latex = LatexBuilder::new();

    for section in &recipe.sections {
        for content in &section.content {
            let instruction = match content {
                Content::Step(step) => step_text(recipe, step),
                Content::Text(text) => text.clone(),
            };

            latex.add_simple_command("item", &instruction);
        }
    }

    latex
}

fn step_text(recipe: &ScaledRecipe, step: &Step) -> String {
    step.items
        .iter()
        .map(|item| match item {
            Item::Text { value } => value.clone(),
            Item::Ingredient { index } => recipe.ingredients[*index].display_name().to_string(),
            Item::Cookware { index } => recipe.cookware[*index].name.clone(),
            Item::Timer { index } => {
                format_timer(&recipe.timers[*index].quantity, &recipe.timers[*index].name)
            }
            Item::InlineQuantity { index } => format_quantity(&recipe.inline_quantities[*index]),
        })
        .collect()
}

fn format_timer(quantity: &Option<Quantity>, name: &Option<String>) -> String {
    match (quantity, name) {
        (Some(qty), Some(name)) => format!("{} ({})", format_quantity(qty), name),
        (Some(qty), None) => format_quantity(qty),
        (None, Some(name)) => name.clone(),
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

    let relative_path = PathBuf::from(collection_name).join(format!("{}.tex", file_stem));

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
