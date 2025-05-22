use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("Failed to read file: {:?}", path.display()))
}

pub fn write_file(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents)
        .with_context(|| format!("Failed to write to file: {}", path.display()))
}

pub fn create_dir_all(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory: {}", path.display()))
}

pub fn list_dir(path: &Path) -> Result<Vec<PathBuf>> {
    Ok(fs::read_dir(path)
        .with_context(|| format!("Failed to read directory: {}", path.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect())
}

pub fn clone_folder_to_target(source: &Path, target: &Path) -> Result<()> {
    create_dir_all(target)?;

    for file in list_dir(source)? {
        let target_path = target.join(file.file_name().context("Invalid source file name")?);

        fs::copy(&file, &target_path).with_context(|| {
            format!(
                "Failed to copy {} to {}",
                file.display(),
                target_path.display()
            )
        })?;
    }

    Ok(())
}
