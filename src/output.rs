//! Defines a rendered `Project`.

use std::{borrow::Cow, fs, path::PathBuf};

use crate::{config::Config, prelude::*, util};

/////////////////////////////////////////////////////////////////////////
// Output definitions
/////////////////////////////////////////////////////////////////////////

/// A rendered file.
pub struct File {
    /// The location of the output file relative to the output directory.
    path: PathBuf,
    /// The raw contents of the file.
    contents: Cow<'static, str>,
}

/// Represents the entire output of our project.
pub struct Output {
    /// The configuration used to control how the project was built.
    config: Config,
    /// Each of the output files.
    files: Vec<File>,
}

/////////////////////////////////////////////////////////////////////////
// Output implementations
/////////////////////////////////////////////////////////////////////////

impl File {
    pub(crate) fn new<S>(path: PathBuf, contents: S) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        let contents = contents.into();
        Self { path, contents }
    }
}

impl Output {
    /// Create a new `Output`.
    pub(crate) fn new(config: Config) -> Self {
        Self {
            config,
            files: Vec::new(),
        }
    }

    /// Get a reference to the `Config`.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Add a new `File` to the `Output`.
    pub(crate) fn push_file(&mut self, file: File) -> &mut Self {
        self.files.push(file);
        self
    }

    /// Write the current `Output` to disk.
    pub fn to_path(&self) -> Result<()> {
        let output_dir = self.config.output_dir();
        util::recreate_dir(&output_dir).with_context(|| {
            format!(
                "failed to recreate output directory `{}`",
                output_dir.display()
            )
        })?;
        for file in &self.files {
            let dst = output_dir.join(&file.path);
            let dir = dst.parent().unwrap();
            fs::create_dir_all(dir)
                .with_context(|| format!("failed to create directory `{}`", dir.display()))?;
            fs::write(&dst, file.contents.as_ref())
                .with_context(|| format!("failed to write file `{}`", dst.display()))?;
        }
        Ok(())
    }
}
