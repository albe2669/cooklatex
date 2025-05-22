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
