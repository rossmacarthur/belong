//! Some useful types and traits that most modules need.

pub use std::result;

pub use anyhow::{anyhow, bail, Context, Error, Result};

pub use crate::util::{FromPath, Join, TomlValueExt};
