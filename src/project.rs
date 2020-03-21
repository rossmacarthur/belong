use std::{path::PathBuf, str};

use anyhow::Context;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::{config::Config, util::TomlValueExt};

/////////////////////////////////////////////////////////////////////////
// Project definitions
/////////////////////////////////////////////////////////////////////////

#[derive(Debug, PartialEq)]
struct Project {
    /// The project's root directory.
    root: PathBuf,
    /// The configuration used to control how the project is built.
    config: Config,
    /// A representation of the project contents in memory.
    pages: Vec<Page>,
}

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
                regex::Regex::new(r"^\s*\+\+\+((?s).*(?-s))\+\+\+\r?\n?((?s).*(?-s))$").unwrap();
        }
        let mut content = s;
        let front_matter = match RE.captures(content) {
            Some(captures) => {
                content = captures.get(2).unwrap().as_str();
                toml::from_str(captures.get(1).unwrap().as_str())
                    .context("failed to parse page front matter")?
            }
            None => FrontMatter::default(),
        };
        Ok(Self {
            front_matter,
            content: content.to_string(),
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
}
