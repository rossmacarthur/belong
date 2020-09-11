//! Configuration for a `Project`.

use std::path::{Path, PathBuf};
use std::str;

use serde::{Deserialize, Serialize};

use crate::prelude::*;

/////////////////////////////////////////////////////////////////////////
// Config definitions
/////////////////////////////////////////////////////////////////////////

/// Project specific configuration.
#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
struct ProjectConfig {
    /// The title of the project.
    title: Option<String>,
    /// The project's authors.
    authors: Option<Vec<String>>,
}

/// The raw config file.
#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct RawConfig {
    /// Project specific configuration.
    #[serde(default)]
    project: ProjectConfig,
    /// The rest of the TOML configuration file.
    #[serde(flatten)]
    rest: toml::Value,
}

/// The overall configuration for a project.
///
/// Contains information from the config file as well as how the `belong` tool
/// was instantiated.
#[derive(Debug, PartialEq)]
pub struct Config {
    /// The project's root directory.
    root_dir: PathBuf,
    /// The configuration as represented on disk.
    inner: RawConfig,
}

/////////////////////////////////////////////////////////////////////////
// Config implementations
/////////////////////////////////////////////////////////////////////////

impl Default for RawConfig {
    fn default() -> Self {
        Self {
            project: ProjectConfig::default(),
            rest: toml::Value::default(),
        }
    }
}

impl str::FromStr for RawConfig {
    type Err = Error;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        Ok(toml::from_str(s)?)
    }
}

impl Config {
    /// Create a new default `Config`.
    pub fn new(root_dir: PathBuf) -> Self {
        Self {
            root_dir,
            inner: RawConfig::default(),
        }
    }

    /// Load a `Config` from disk.
    pub fn from_path(root_dir: PathBuf) -> Result<Self> {
        let path = root_dir.join("belong.toml");
        let inner = RawConfig::from_path(&path)
            .with_context(|| format!("failed to load config file `{}`", path.display()))?;
        Ok(Self { root_dir, inner })
    }

    /// Get the root directory.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// The path to config file.
    pub fn path(&self) -> PathBuf {
        self.root_dir.join("belong.toml")
    }

    /// Convert a `Config` to raw TOML bytes.
    pub fn to_toml_vec(&self) -> Result<Vec<u8>> {
        Ok(toml::to_vec(&self.inner)?)
    }

    /// Return a type that implements `Serialize`. This can be used to serialize
    /// the `Config` to JSON.
    pub fn as_context(&self) -> &impl Serialize {
        &self.inner
    }

    /// Get the src directory.
    pub fn src_dir(&self) -> PathBuf {
        self.root_dir.join("src")
    }

    /// Get the theme directory.
    pub fn theme_dir(&self) -> PathBuf {
        self.root_dir.join("theme")
    }

    /// Get the output directory.
    pub fn output_dir(&self) -> PathBuf {
        self.root_dir.join("output")
    }

    /// Get a mutable reference to the project title.
    pub fn title_mut(&mut self) -> &mut Option<String> {
        &mut self.inner.project.title
    }

    /// Get a mutable reference to the project authors.
    pub fn authors_mut(&mut self) -> &mut Option<Vec<String>> {
        &mut self.inner.project.authors
    }
}

/////////////////////////////////////////////////////////////////////////
// Unit tests
/////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    use toml::toml;

    #[test]
    fn raw_config_from_str_empty() {
        let raw_config: RawConfig = toml::from_str("").unwrap();
        assert_eq!(raw_config, RawConfig::default());
    }

    #[test]
    fn raw_config_from_str_project() {
        let content = r#"
            [project]
            title = "My Blog"
        "#;
        let raw_config: RawConfig = toml::from_str(content).unwrap();
        assert_eq!(
            raw_config,
            RawConfig {
                project: ProjectConfig {
                    title: Some("My Blog".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            }
        );
    }

    #[test]
    fn raw_config_from_str_rest() {
        let content = r#"
            [plugin]
            another = 5
        "#;
        let raw_config: RawConfig = toml::from_str(content).unwrap();
        assert_eq!(
            raw_config,
            RawConfig {
                rest: toml! {
                    [plugin]
                    another = 5
                },
                ..Default::default()
            }
        );
    }

    #[test]
    fn raw_config_from_str_both() {
        let content = r#"
            [project]
            title = "My Blog"

            [plugin]
            another = 5
        "#;
        let raw_config: RawConfig = toml::from_str(content).unwrap();
        assert_eq!(
            raw_config,
            RawConfig {
                project: ProjectConfig {
                    title: Some("My Blog".to_string()),
                    ..Default::default()
                },
                rest: toml! {
                    [plugin]
                    another = 5
                },
            }
        );
    }
}
