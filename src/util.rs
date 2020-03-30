use std::{fs, io, path::Path, str::FromStr};

use anyhow::Context;

pub trait TomlValueExt {
    fn default() -> Self;
}

impl TomlValueExt for toml::Value {
    fn default() -> Self {
        Self::Table(toml::value::Table::default())
    }
}

pub trait FromPath
where
    Self: Sized,
{
    /// Read and parse an object from a file path.
    fn from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Self>;
}

impl<T> FromPath for T
where
    T: FromStr<Err = anyhow::Error>,
{
    /// Read an object from a file path and parse it using `FromStr`.
    fn from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let obj = fs::read_to_string(path)
            .with_context(|| format!("failed to read file `{}`", path.display()))?
            .parse()
            .context("failed to parse file contents")?;
        Ok(obj)
    }
}

/// Completely delete and recreate a directory.
pub fn recreate_dir<P: AsRef<Path>>(dir: P) -> anyhow::Result<()> {
    let dir = dir.as_ref();
    if let Err(e) = fs::remove_dir_all(&dir) {
        if e.kind() != io::ErrorKind::NotFound {
            return Err(e)
                .with_context(|| format!("failed to remove directory `{}`", dir.display()));
        }
    }
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create directory `{}`", dir.display()))?;
    Ok(())
}
