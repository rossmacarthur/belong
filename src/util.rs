//! General purpose utility traits and functions.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::str::FromStr;

use crate::prelude::*;

/// A trait to provide a `default()` function for [`toml::Value`].
///
/// [`toml::Value`]: ../../toml/value/enum.Value.html
pub trait TomlValueExt {
    fn default() -> Self;
}

impl TomlValueExt for toml::Value {
    /// Return an empty TOML table.
    fn default() -> Self {
        Self::Table(toml::value::Table::default())
    }
}

/// A trait to implement reading something from disk.
///
/// This trait is auto implemented for all types that implement [`FromStr`].
///
/// [`FromStr`]: https://doc.rust-lang.org/std/str/trait.FromStr.html
pub trait FromPath
where
    Self: Sized,
{
    /// Read and parse an object from a file path.
    fn from_path<P: AsRef<Path>>(path: P) -> Result<Self>;
}

impl<T> FromPath for T
where
    T: FromStr<Err = Error>,
{
    /// Read an object from a file path and parse it using `FromStr`.
    fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let obj = fs::read_to_string(path)
            .context("failed to read file")?
            .parse()
            .context("failed to parse file contents")?;
        Ok(obj)
    }
}

/// Copy of the [`std::slice::Join`] trait so we can implement it for standard
/// library types like `&[&OsStr]`.
///
/// See [#rust-lang/#61133].
///
/// [`std::slice::Join`]: https://doc.rust-lang.org/std/slice/trait.Join.html
/// [#rust-lang/#61133]: https://github.com/rust-lang/rust/issues/61133
pub trait Join<S> {
    type Output;
    fn join(self, sep: S) -> Self::Output;
}

impl<S> Join<S> for &[&OsStr]
where
    S: AsRef<OsStr>,
{
    type Output = OsString;

    /// Join an `OsString` using the given separator and return the new
    /// `OsString`.
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
pub fn recreate_dir<P>(dir: P) -> Result<()>
where
    P: AsRef<Path>,
{
    let dir = dir.as_ref();
    if let Err(e) = fs::remove_dir_all(&dir) {
        if e.kind() != io::ErrorKind::NotFound {
            return Err(e).context("failed to remove directory");
        }
    }
    fs::create_dir_all(&dir).context("failed to create directory")?;
    Ok(())
}

/// Create and write to a file if it does not exist.
pub fn write_new<P, C>(path: P, contents: C) -> Result<()>
where
    P: AsRef<Path>,
    C: AsRef<[u8]>,
{
    let path = path.as_ref();
    let contents = contents.as_ref();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("failed to create file `{}`", path.display()))?;
    file.write(contents)
        .with_context(|| format!("failed to write to file `{}`", path.display()))?;
    Ok(())
}
