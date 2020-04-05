use std::{
    ffi::{OsStr, OsString},
    fs, io,
    path::Path,
    str::FromStr,
};

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
            .context("failed to read file")?
            .parse()
            .context("failed to parse file contents")?;
        Ok(obj)
    }
}

/// Copy of the `std::slice::Join` trait so we can implement it for standard
/// library types like `&[&OsStr]`.
///
/// See https://github.com/rust-lang/rust/issues/61133.
pub trait Join<S> {
    type Output;
    fn join(self, sep: S) -> Self::Output;
}

impl<S> Join<S> for &[&OsStr]
where
    S: AsRef<OsStr>,
{
    type Output = OsString;

    fn join(self, sep: S) -> Self::Output {
        let sep = sep.as_ref();
        let mut result = OsString::new();
        match self.split_first() {
            Some((first, rest)) => {
                result.push(first);
                for element in rest {
                    result.push(sep);
                    result.push(element);
                }
                result
            }
            None => result,
        }
    }
}

/// Completely delete and recreate a directory.
pub fn recreate_dir<P: AsRef<Path>>(dir: P) -> anyhow::Result<()> {
    let dir = dir.as_ref();
    if let Err(e) = fs::remove_dir_all(&dir) {
        if e.kind() != io::ErrorKind::NotFound {
            return Err(e).context("failed to remove directory");
        }
    }
    fs::create_dir_all(&dir).context("failed to create directory")?;
    Ok(())
}
