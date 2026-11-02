//! Free-form `servings` metadata.
//!
//! Cooklang expects `servings` to be a plain number, but many recipes are
//! better described by what they make: `16 boller`, `1 tærte`,
//! `2 brød eller 15 boller`, `ca. 20 stk`. These are classified as either
//! servings (a number/range of people or portions) or a yield (anything else).

use crate::locale::Locale;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServingsKind {
    /// Feeds a number of people, e.g. `4`, `4-6` or `6 portioner`
    Servings,
    /// Makes a number of something, e.g. `16 boller` or `0.5 liter`
    Yield,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServingsInfo {
    pub kind: ServingsKind,
    /// Text to show, without LaTeX escaping
    pub text: String,
}

impl ServingsInfo {
    pub fn parse(raw: &str, locale: &Locale) -> Self {
        let raw = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        let words: Vec<&str> = raw.split(' ').collect();

        let Some(num_idx) = words.iter().position(|w| is_amount(w)) else {
            // No number at all, e.g. "en stor portion"
            return Self {
                kind: ServingsKind::Yield,
                text: locale.localize_number(&raw),
            };
        };

        let rest = &words[num_idx + 1..];
        let is_servings = rest.is_empty() || (rest.len() == 1 && locale.is_serving_unit(rest[0]));

        let shown: &[&str] = if is_servings {
            &words[..=num_idx]
        } else {
            &words
        };

        Self {
            kind: if is_servings {
                ServingsKind::Servings
            } else {
                ServingsKind::Yield
            },
            text: locale.localize_number(&shown.join(" ")),
        }
    }
}

/// A number or range: `4`, `4-6`, `0.5`, `1/2`, `2,5`
fn is_amount(word: &str) -> bool {
    word.chars().next().is_some_and(|c| c.is_ascii_digit())
        && word
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '/' | '-' | '–'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> (ServingsKind, String) {
        let locale = Locale::load("da", None).unwrap();
        let s = ServingsInfo::parse(raw, &locale);
        (s.kind, s.text)
    }

    use ServingsKind::*;

    #[test]
    fn servings() {
        assert_eq!(parse("4"), (Servings, "4".into()));
        assert_eq!(parse("4-6"), (Servings, "4-6".into()));
        assert_eq!(parse("4-6 personer"), (Servings, "4-6".into()));
        assert_eq!(parse("6 Portioner"), (Servings, "6".into()));
        assert_eq!(parse("ca. 4 personer"), (Servings, "ca. 4".into()));
    }

    #[test]
    fn yields() {
        assert_eq!(parse("16 boller"), (Yield, "16 boller".into()));
        assert_eq!(parse("ca. 20 stk"), (Yield, "ca. 20 stk".into()));
        assert_eq!(parse("0.5 liter"), (Yield, "0,5 liter".into()));
        assert_eq!(
            parse("2 brød eller 15 boller"),
            (Yield, "2 brød eller 15 boller".into())
        );
        assert_eq!(
            parse("1 stor focaccia (8-12 stykker)"),
            (Yield, "1 stor focaccia (8-12 stykker)".into())
        );
        assert_eq!(parse("en stor gryde"), (Yield, "en stor gryde".into()));
    }
}
