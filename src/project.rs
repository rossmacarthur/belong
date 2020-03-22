use std::{path::PathBuf, str};

use anyhow::Context;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    util::{FromPath, TomlValueExt},
};

/////////////////////////////////////////////////////////////////////////
// Project definitions
/////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct FrontMatter {
    /// The title for this page.
    title: Option<String>,
    /// The description for this page.
    description: Option<String>,
    /// The date this page was written.
    date: Option<chrono::NaiveDate>,
    /// The rest of the TOML front matter.
    #[serde(flatten)]
    rest: toml::Value,
}

#[derive(Debug, Default, PartialEq)]
pub struct Page {
    /// Front matter for the page.
    front_matter: FrontMatter,
    /// The contents of the page.
    content: String,
}

#[derive(Debug, PartialEq)]
pub struct Project {
    /// The project's root directory.
    root_dir: PathBuf,
    /// The configuration used to control how the project is built.
    config: Config,
    /// A representation of the project contents in memory.
    pages: Vec<Page>,
}

/////////////////////////////////////////////////////////////////////////
// Project implementations
/////////////////////////////////////////////////////////////////////////

impl Default for FrontMatter {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            date: None,
            rest: toml::Value::default(),
        }
    }
}

impl str::FromStr for Page {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: regex::Regex =
                regex::Regex::new(r"^\s*\+\+\+((?s).*(?-s))\+\+\+(\r?\n)+((?s).*(?-s))$").unwrap();
        }
        let mut content = s;
        let front_matter = match RE.captures(content) {
            Some(captures) => {
                content = captures.get(3).unwrap().as_str();
                toml::from_str(captures.get(1).unwrap().as_str())
                    .context("failed to parse front matter")?
            }
            None => FrontMatter::default(),
        };
        Ok(Self {
            front_matter,
            content: content.to_string(),
        })
    }
}

impl Project {
    /// Create a new `Project` from the given directory.
    pub fn from_path(root_dir: PathBuf) -> anyhow::Result<Self> {
        let config_file = root_dir.join("belong.toml");

        // Load the config file from disk.
        let config = if config_file.exists() {
            Config::from_path(&config_file).with_context(|| {
                format!("failed to load config file `{}`", config_file.display())
            })?
        } else {
            Config::default()
        };

        // Load all the pages from disk.
        let src_dir = root_dir.join("src");
        let pages: Vec<Page> = walkdir::WalkDir::new(src_dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().map(|s| s == "md").unwrap_or(false))
            .map(|e| {
                Page::from_path(e.path())
                    .with_context(|| format!("failed to load page `{}`", e.path().display()))
            })
            .collect::<Result<_, _>>()?;

        Ok(Self {
            root_dir,
            config,
            pages,
        })
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
    fn page_from_str_empty() {
        let page: Page = "".parse().unwrap();
        assert_eq!(page, Page::default());
    }

    #[test]
    fn page_from_str_no_front_matter() {
        let page: Page = "testing...".parse().unwrap();
        assert_eq!(
            page,
            Page {
                content: "testing...".to_string(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn page_from_str_empty_front_matter() {
        let content = r#"
+++
+++
testing...
"#;
        let page: Page = content.parse().unwrap();
        assert_eq!(
            page,
            Page {
                content: "testing...\n".to_string(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn page_from_str_basic_front_matter() {
        let content = r#"
+++
title = "Hello World!"
date = "2020-03-21"
+++
testing...
"#;
        let page: Page = content.parse().unwrap();
        assert_eq!(
            page,
            Page {
                content: "testing...\n".to_string(),
                front_matter: FrontMatter {
                    title: Some("Hello World!".to_string()),
                    date: Some(chrono::NaiveDate::from_ymd(2020, 3, 21)),
                    ..Default::default()
                }
            }
        );
    }

    #[test]
    fn page_from_str_extra_front_matter() {
        let content = r#"
+++
title = "Hello World!"
description = "My first post!"
date = "2020-03-21"
testing_int = 5
testing_str = "hello"
+++
testing...
"#;
        let page: Page = content.parse().unwrap();
        assert_eq!(
            page,
            Page {
                content: "testing...\n".to_string(),
                front_matter: FrontMatter {
                    title: Some("Hello World!".to_string()),
                    description: Some("My first post!".to_string()),
                    date: Some(chrono::NaiveDate::from_ymd(2020, 3, 21)),
                    rest: toml! {
                        testing_int = 5
                        testing_str = "hello"
                    }
                }
            }
        );
    }

    #[test]
    fn project_from_path_empty() {
        let root_dir = tempfile::tempdir().unwrap().into_path();
        std::fs::create_dir(root_dir.join("src")).unwrap();

        let project = Project::from_path(root_dir.clone()).unwrap();
        assert_eq!(
            project,
            Project {
                root_dir,
                config: Config::default(),
                pages: Vec::new(),
            }
        )
    }

    #[test]
    fn project_from_path_bad_config() {
        let root_dir = tempfile::tempdir().unwrap().into_path();
        std::fs::write(root_dir.join("belong.toml"), "very bad toml").unwrap();
        let err = Project::from_path(root_dir.clone()).unwrap_err();
        assert_eq!(
            format!("{:?}", err),
            format!(
                r#"failed to load config file `{}/belong.toml`

Caused by:
    0: failed to parse file contents
    1: expected an equals, found an identifier at line 1 column 6"#,
                root_dir.display()
            )
        );
    }

    #[test]
    fn project_from_path_bad_page() {
        let root_dir = tempfile::tempdir().unwrap().into_path();
        std::fs::create_dir(root_dir.join("src")).unwrap();
        let page_content = r#"
+++
bad toml
+++
testing...
"#;
        std::fs::write(root_dir.join("src/test.md"), &page_content).unwrap();
        let err = Project::from_path(root_dir.clone()).unwrap_err();
        assert_eq!(
            format!("{:?}", err),
            format!(
                r#"failed to load page `{}/src/test.md`

Caused by:
    0: failed to parse file contents
    1: failed to parse front matter
    2: expected an equals, found an identifier at line 2 column 5"#,
                root_dir.display()
            )
        );
    }

    #[test]
    fn project_from_path_custom_config_and_pages() {
        let root_dir = tempfile::tempdir().unwrap().into_path();
        std::fs::create_dir(root_dir.join("src")).unwrap();
        let config_content = toml!(
            [project]
            title = "My Blog"

            [plugin]
            another = 5
        )
        .to_string();
        std::fs::write(root_dir.join("belong.toml"), &config_content).unwrap();

        let page_content = r#"
+++
title = "Hello World!"
date = "2020-03-21"
+++
testing...
"#;
        std::fs::write(root_dir.join("src/test.md"), &page_content).unwrap();

        let project = Project::from_path(root_dir.clone()).unwrap();
        assert_eq!(
            project,
            Project {
                root_dir,
                config: str::FromStr::from_str(&config_content).unwrap(),
                pages: vec![str::FromStr::from_str(&page_content).unwrap()]
            }
        )
    }
}
