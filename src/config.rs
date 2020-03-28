use std::str;

use serde::{Deserialize, Serialize};

use crate::util::TomlValueExt;

/////////////////////////////////////////////////////////////////////////
// Config definitions
/////////////////////////////////////////////////////////////////////////

/// Project specific configuration.
#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
struct ProjectConfig {
    /// The title of the project.
    title: Option<String>,
}

/// The overall configuration for a project.
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct Config {
    /// Project specific configuration.
    #[serde(default)]
    project: ProjectConfig,
    /// The rest of the TOML configuration file.
    #[serde(flatten)]
    rest: toml::Value,
}

/////////////////////////////////////////////////////////////////////////
// Config implementations
/////////////////////////////////////////////////////////////////////////

impl Default for Config {
    fn default() -> Self {
        Self {
            project: ProjectConfig::default(),
            rest: toml::Value::default(),
        }
    }
}

impl str::FromStr for Config {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(toml::from_str(s)?)
    }
}

/////////////////////////////////////////////////////////////////////////
// Unit tests
/////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {
    use super::*;

    use toml::toml;

    #[test]
    fn config_from_str_empty() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn config_from_str_project() {
        let content = r#"
            [project]
            title = "My Blog"
        "#;
        let config: Config = toml::from_str(content).unwrap();
        assert_eq!(
            config,
            Config {
                project: ProjectConfig {
                    title: Some("My Blog".to_string()),
                },
                ..Default::default()
            }
        );
    }

    #[test]
    fn config_from_str_rest() {
        let content = r#"
            [plugin]
            another = 5
        "#;
        let config: Config = toml::from_str(content).unwrap();
        assert_eq!(
            config,
            Config {
                rest: toml! {
                    [plugin]
                    another = 5
                },
                ..Default::default()
            }
        );
    }

    #[test]
    fn config_from_str_both() {
        let content = r#"
            [project]
            title = "My Blog"

            [plugin]
            another = 5
        "#;
        let config: Config = toml::from_str(content).unwrap();
        assert_eq!(
            config,
            Config {
                project: ProjectConfig {
                    title: Some("My Blog".to_string()),
                },
                rest: toml! {
                    [plugin]
                    another = 5
                },
            }
        );
    }
}
