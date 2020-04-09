use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs, io,
    io::Write,
    path::Path,
    str::FromStr,
};

use anyhow::Context;
use serde_json as json;

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
pub fn recreate_dir<P>(dir: P) -> anyhow::Result<()>
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

/// Create and write to a file if it doesn't exist.
pub fn write_new<P, C>(path: P, contents: C) -> anyhow::Result<()>
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

/// A Tera template filter to filter array values.
///
/// This is copied from Tera source code to allow `value` arguments to be null.
/// In the case where `value` arguments are null, only null values will be
/// filtered out.
pub fn filter(
    value: &json::Value,
    args: &HashMap<String, json::Value>,
) -> tera::Result<json::Value> {
    let arr = tera::try_get_value!("filter", "value", Vec<json::Value>, value);
    let key = match args.get("attribute") {
        Some(val) => tera::try_get_value!("filter", "attribute", String, val),
        None => {
            return Err(tera::Error::msg(
                "The `filter` filter has to have an `attribute` argument",
            ))
        }
    };
    let value = args.get("value").unwrap_or(&json::Value::Null);
    let json_pointer = ["/", &key.replace(".", "/")].concat();
    let filtered = arr
        .into_iter()
        .filter(|v| {
            let val = v.pointer(&json_pointer).unwrap_or(&json::Value::Null);
            if value.is_null() {
                !val.is_null()
            } else {
                val == value
            }
        })
        .collect::<Vec<_>>();
    Ok(tera::to_value(filtered).unwrap())
}
