use std::io::Write;
use std::path::Path;
use anyhow::{Context, Result};
pub mod templates;
pub mod fs;
pub mod walk;
use std::borrow::Cow;

pub fn process_text_file<P, C, T>(path: P, should_process: C, transform: T) -> Result<()>
where
    P: AsRef<Path>,
    C: Fn(&str) -> bool,
    T: FnOnce(&str) -> Cow<str>,
{
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    if !should_process(&content) {
        return Ok(());
    }

    let transformed = transform(&content);

    if transformed != content {
        let temp_path = path.with_extension("tmp");
        {
            let mut file = std::fs::File::create(&temp_path)
                .with_context(|| format!("Failed to create temporary file: {}", temp_path.display()))?;
            file.write_all(transformed.as_bytes())
                .with_context(|| format!("Failed to write to temporary file: {}", temp_path.display()))?;
        }
        std::fs::rename(&temp_path, path)
            .with_context(|| format!("Failed to rename temporary file to: {}", path.display()))?;
    }

    Ok(())
}
