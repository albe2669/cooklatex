//! Localisation of the strings cooklatex writes into the generated LaTeX.
//!
//! Locales are TOML files. The built-in ones live in `locales/` and are
//! embedded into the binary; a custom file can be merged on top of the
//! selected locale to tweak or add strings.

use std::{collections::HashMap, path::Path};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::latex::sanitize_latex;

const BUILTIN: &[(&str, &str)] = &[
    ("en", include_str!("../locales/en.toml")),
    ("da", include_str!("../locales/da.toml")),
];

/// Name of the file written to the output directory, `\input` by the template.
pub const LOCALE_TEX_FILE: &str = "cooklatex-locale.tex";

#[derive(Debug, Clone, Deserialize)]
pub struct Locale {
    pub babel: String,
    pub decimal_separator: String,
    pub labels: Labels,
    pub time: TimeUnits,
    pub difficulty: Difficulty,
    pub servings: ServingsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Labels {
    pub servings: String,
    #[serde(rename = "yield")]
    pub yield_: String,
    pub prep: String,
    pub cooking: String,
    pub difficulty: String,
    pub optional: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TimeUnits {
    pub minute: String,
    pub minutes: String,
    pub hour: String,
    pub hours: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Difficulty {
    /// Key (or literal text) used when a recipe has no `difficulty` metadata
    pub default: String,
    /// Translations for difficulty keys, e.g. `easy = "Let"`
    #[serde(flatten)]
    pub levels: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServingsConfig {
    /// Units that mean "people/portions"
    pub units: Vec<String>,
}

impl Locale {
    pub fn available() -> impl Iterator<Item = &'static str> {
        BUILTIN.iter().map(|(code, _)| *code)
    }

    /// Load a built-in locale, optionally merging a custom TOML file on top.
    pub fn load(lang: &str, custom: Option<&Path>) -> Result<Self> {
        let Some((_, builtin)) = BUILTIN
            .iter()
            .find(|(code, _)| code.eq_ignore_ascii_case(lang))
        else {
            bail!(
                "Unknown language '{lang}'. Built-in languages: {}. \
                 Use --locale-file to provide your own.",
                Self::available().collect::<Vec<_>>().join(", ")
            );
        };

        let mut value: toml::Table = toml::from_str(builtin).expect("built-in locale is valid");

        if let Some(path) = custom {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("Cannot read locale file: {}", path.display()))?;
            let overrides: toml::Table = toml::from_str(&text)
                .with_context(|| format!("Invalid locale file: {}", path.display()))?;
            merge(&mut value, overrides);
        }

        toml::Value::Table(value)
            .try_into()
            .context("Locale is missing required keys")
    }

    pub fn format_minutes(&self, minutes: u64) -> String {
        let t = &self.time;
        let unit =
            |n: u64, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });

        if minutes < 60 {
            return unit(minutes, &t.minute, &t.minutes);
        }
        let hours = unit(minutes / 60, &t.hour, &t.hours);
        match minutes % 60 {
            0 => hours,
            m => format!("{hours} {}", unit(m, &t.minute, &t.minutes)),
        }
    }

    /// Translate a difficulty key (`easy`, `moderate`, ...). Unknown values
    /// are passed through so recipes can use free text.
    pub fn difficulty(&self, value: Option<&str>) -> String {
        let value = value.unwrap_or(&self.difficulty.default);
        self.difficulty
            .levels
            .get(&value.trim().to_lowercase())
            .cloned()
            .unwrap_or_else(|| value.to_string())
    }

    /// Replace `.` decimal points between digits with the locale's separator.
    pub fn localize_number(&self, text: &str) -> String {
        if self.decimal_separator == "." {
            return text.to_string();
        }
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::with_capacity(text.len());
        for (i, c) in chars.iter().enumerate() {
            let between_digits = i > 0
                && chars[i - 1].is_ascii_digit()
                && chars.get(i + 1).is_some_and(char::is_ascii_digit);
            if *c == '.' && between_digits {
                out.push_str(&self.decimal_separator);
            } else {
                out.push(*c);
            }
        }
        out
    }

    pub fn is_serving_unit(&self, unit: &str) -> bool {
        let unit = unit.trim().to_lowercase();
        self.servings.units.iter().any(|u| u.to_lowercase() == unit)
    }

    /// The LaTeX file defining the template's localised macros.
    pub fn to_tex(&self) -> String {
        let l = &self.labels;
        let cmd = |name: &str, value: &str| {
            format!(
                "\\providecommand{{\\{name}}}{{{}}}\n",
                sanitize_latex(value)
            )
        };

        let mut tex = String::from("% Generated by cooklatex. Do not edit.\n");
        tex += &format!("\\def\\cllanguage{{{}}}\n", self.babel);
        tex += &format!("\\PassOptionsToPackage{{{}}}{{babel}}\n", self.babel);
        tex += &format!("\\PassOptionsToPackage{{{}}}{{isodate}}\n", self.babel);
        tex += &cmd("clservingslabel", &l.servings);
        tex += &cmd("clyieldlabel", &l.yield_);
        tex += &cmd("clpreplabel", &l.prep);
        tex += &cmd("clcookinglabel", &l.cooking);
        tex += &cmd("cldifficultylabel", &l.difficulty);
        tex += &cmd("cloptionallabel", &l.optional);
        tex
    }
}

fn merge(base: &mut toml::Table, overrides: toml::Table) {
    for (key, value) in overrides {
        match (base.get_mut(&key), value) {
            (Some(toml::Value::Table(b)), toml::Value::Table(o)) => merge(b, o),
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_load() {
        for code in Locale::available() {
            Locale::load(code, None).unwrap();
        }
    }

    #[test]
    fn danish_formatting() {
        let da = Locale::load("da", None).unwrap();
        assert_eq!(da.format_minutes(1), "1 min");
        assert_eq!(da.format_minutes(60), "1 time");
        assert_eq!(da.format_minutes(150), "2 timer 30 min");
        assert_eq!(da.difficulty(None), "Middel");
        assert_eq!(da.difficulty(Some("Hard")), "Svær");
        assert_eq!(
            da.difficulty(Some("Kræver tålmodighed")),
            "Kræver tålmodighed"
        );
        assert_eq!(da.localize_number("0.5 liter, ca. 2"), "0,5 liter, ca. 2");
    }

    #[test]
    fn english_formatting() {
        let en = Locale::load("EN", None).unwrap();
        assert_eq!(en.format_minutes(45), "45 mins");
        assert_eq!(en.format_minutes(61), "1 hr 1 min");
        assert_eq!(en.localize_number("0.5"), "0.5");
    }

    #[test]
    fn unknown_language() {
        assert!(Locale::load("xx", None).is_err());
    }
}
