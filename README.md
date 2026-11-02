# Cooklatex

Cooklatex is a [Cooklang](https://cooklang.org/) to LaTeX transpiler, written in Rust. It uses the [cooklang-chef](https://github.com/Zheoni/cooklang-chef) project to parse Cooklang files and convert them to LaTeX documents. 

## Compile
To compile the project, you need to have [Rust](https://www.rust-lang.org/) installed. You can install Rust using [rustup](https://rustup.rs/). Once you have Rust installed, you can compile the project using the following command:

```bash
cargo build --release
```

## Usage
To use the transpiler, you can run the following command:
```bash
cargo run -- --latex-dir <LATEX_DIR> --latex-out-dir <LATEX_OUT_DIR> [COLLECTIONS]
```

Where `<LATEX_DIR>` is the path to the directory containing the LaTeX templates (see [latex-example](./latex-example)), and `<LATEX_OUT_DIR>` is the path to the directory where you want to save the generated LaTeX files.

The `COLLECTIONS` argument is a list of Cooklang directories containing Cooklang files.

### Options
| Option | Description |
| --- | --- |
| `-L, --lang <LANG>` | Language of the generated text. Built-in: `en` (default), `da`. |
| `--locale-file <FILE>` | TOML file merged on top of the selected language, to tweak strings or add a new language. See [locales/en.toml](./locales/en.toml) for the keys. |
| `-c, --convert <SYSTEM>` | Convert quantities to a unit system (`metric` or `imperial`). |
| `-u, --units-file <FILE>` | Custom units file in TOML format. |

## Languages
Every piece of text cooklatex generates (metadata labels, time units, difficulty, decimal separator, "optional") comes from a locale in [locales](./locales). The locale is also written to `cooklatex-locale.tex` in the output directory, which the template `\input`s. It defines:

- `\clservingslabel`, `\clyieldlabel`, `\clpreplabel`, `\clcookinglabel`, `\cldifficultylabel`, `\cloptionallabel`
- the babel/isodate language, via `\PassOptionsToPackage`

So a template should `\input{cooklatex-locale}` before loading `babel`, and use these macros instead of hard-coded text. To add a language, copy `locales/en.toml`, translate it and register it in `src/locale.rs` (or pass it with `--locale-file`).

## Servings
Cooklang wants `servings` to be a number, but cooklatex also accepts free-form values:

| `servings:` | Shown as (`--lang da`) |
| --- | --- |
| `4`, `4-6` | Portioner 4, Portioner 4–6 |
| `6 portioner`, `4-6 personer` | Portioner 6, Portioner 4–6 |
| `16 boller`, `1 tærte`, `ca. 20 stk`, `2 brød eller 15 boller` | Giver 16 boller, Giver 1 tærte, ... |

A number or range, optionally followed by one of the locale's `servings.units` (people/portions), is shown under the servings label. Anything else is shown as-is under the yield label. The template gets the label macro as the first argument of `\recipemeta{label}{servings}{prep}{cooking}{difficulty}`.

An optional `difficulty` metadata key (`easy`, `moderate`, `hard`, or free text) is also supported; it defaults to `moderate`.
