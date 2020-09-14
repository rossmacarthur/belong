//! Core application code.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str;

use regex_macro::regex;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::output::Output;
use crate::prelude::*;
use crate::theme::Theme;
use crate::util;

/////////////////////////////////////////////////////////////////////////
// Project definitions
/////////////////////////////////////////////////////////////////////////

/// Represents the TOML front matter of a Markdown document.
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct FrontMatter {
    /// The title for this page.
    title: Option<String>,
    /// The description for this page.
    description: Option<String>,
    /// The date this page was written.
    date: Option<chrono::NaiveDate>,
    /// The type of page this is.
    kind: Option<String>,
    /// The rest of the TOML front matter.
    #[serde(flatten)]
    rest: toml::Value,
}

/// A raw page on disk.
#[derive(Debug, Default, PartialEq)]
struct RawPage {
    /// Front matter for the raw page.
    front_matter: FrontMatter,
    /// The contents of the raw page.
    contents: String,
}

/// Represents a Markdown page in our project.
#[derive(Debug, Default, PartialEq)]
pub struct Page {
    /// The location of the page's source file relative to the `src` directory.
    pub path: PathBuf,
    /// Front matter for the page.
    pub front_matter: FrontMatter,
    /// The contents of the page.
    pub contents: String,
}

/// A builder to initialize a new project.
#[derive(Debug)]
pub struct Builder {
    /// The config used to control how the project is built.
    config: Config,
    /// Whether to initialize a .gitignore file.
    gitignore: bool,
}

/// Represents our entire project.
#[derive(Debug, PartialEq)]
pub struct Project {
    /// The config used to control how the project is built.
    config: Config,
    /// The theme to use when rendering the project's HTML.
    theme: Theme,
    /// Each of the project's pages.
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
            kind: None,
            rest: toml::Value::default(),
        }
    }
}

impl fmt::Display for FrontMatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "+++\n{}+++\n", toml::to_string_pretty(self).unwrap())
    }
}

impl fmt::Display for RawPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\n{}", self.front_matter, self.contents)
    }
}

impl str::FromStr for RawPage {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = regex!(r"^\s*\+\+\+((?s).*(?-s))\+\+\+(\r?\n)+((?s).*(?-s))$");
        let mut contents = s;
        let front_matter = match re.captures(contents) {
            Some(captures) => {
                contents = captures.get(3).unwrap().as_str();
                toml::from_str(captures.get(1).unwrap().as_str())
                    .context("failed to parse front matter")?
            }
            None => FrontMatter::default(),
        };
        Ok(Self {
            front_matter,
            contents: contents.to_string(),
        })
    }
}

impl Page {
    /// Load a `Page` from the given path.
    fn from_path(src_dir: &Path, full_path: &Path) -> Result<Self> {
        let raw_page = RawPage::from_path(&full_path)?;
        let path = full_path.strip_prefix(&src_dir).unwrap().to_path_buf();
        Ok(Self {
            path,
            front_matter: raw_page.front_matter,
            contents: raw_page.contents,
        })
    }
}

impl Builder {
    /// Create a new `Builder`.
    pub fn new<P>(root_dir: P) -> Self
    where
        P: Into<PathBuf>,
    {
        Self {
            config: Config::new(root_dir.into()),
            gitignore: true,
        }
    }

    /// Update the title for the project.
    pub fn title<S>(&mut self, title: S) -> &mut Self
    where
        S: Into<String>,
    {
        *self.config.title_mut() = Some(title.into());
        self
    }

    /// Add an author for the project.
    pub fn author<S>(&mut self, author: S) -> &mut Self
    where
        S: Into<String>,
    {
        self.config
            .authors_mut()
            .get_or_insert_with(|| Vec::with_capacity(1))
            .push(author.into());
        self
    }

    /// Whether to create a `.gitignore` file.
    pub fn gitignore(&mut self, gitignore: bool) -> &mut Self {
        self.gitignore = gitignore;
        self
    }

    /// Returns the default `hello-world.md` page.
    fn generate_hello_world_page() -> RawPage {
        RawPage {
            front_matter: FrontMatter {
                title: Some("Hello World!".to_string()),
                date: Some(chrono::Local::today().naive_local()),
                kind: Some("post".to_string()),
                ..Default::default()
            },
            contents: r#"Hello World! This is the first page on my site.

I wrote some Rust code for the occasion:

```rust
fn main() {
    println!("Hello, world!");
}
```
"#
            .to_string(),
        }
    }

    /// Initialize the project by writing the files to disk.
    pub fn init(&self) -> Result<()> {
        // Create directory structure.
        let src_dir = self.config.src_dir();
        fs::create_dir_all(&src_dir)
            .with_context(|| format!("failed to create src directory `{}`", src_dir.display()))?;

        if self.gitignore {
            // Create .gitignore file.
            util::write_new(self.config.root_dir().join(".gitignore"), "public\n")?;
        }

        // Create config file.
        util::write_new(self.config.path(), self.config.to_toml_vec()?)?;

        // Create Hello World! post.
        util::write_new(
            src_dir.join("hello-world.md"),
            Self::generate_hello_world_page().to_string(),
        )?;

        Ok(())
    }
}

impl Project {
    /// Load a `Project` from the given directory.
    pub fn from_path<P>(root_dir: P) -> Result<Self>
    where
        P: Into<PathBuf>,
    {
        let config = Config::from_path(root_dir.into()).context("failed to load config")?;
        let theme = Theme::from_path(&config.theme_dir()).context("failed to load theme")?;

        // Finally load all the the pages from disk.
        let src_dir = config.src_dir();
        let pages: Vec<_> = walkdir::WalkDir::new(&src_dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().map(|s| s == "md").unwrap_or(false))
            .map(|e| {
                Page::from_path(&src_dir, e.path())
                    .with_context(|| format!("failed to load page `{}`", e.path().display()))
            })
            .collect::<Result<_, _>>()?;

        Ok(Self {
            config,
            theme,
            pages,
        })
    }

    /// Render a `Project`.
    pub fn render(self) -> Result<Output> {
        Ok(self
            .theme
            .render(self.config, self.pages)
            .context("failed to render project")?)
    }
}

/////////////////////////////////////////////////////////////////////////
// Unit tests
/////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    use std::panic;

    use toml::toml;

    #[test]
    fn raw_page_from_str_empty() {
        let raw_page: RawPage = "".parse().unwrap();
        assert_eq!(raw_page, RawPage::default());
    }

    #[test]
    fn raw_page_from_str_no_front_matter() {
        let raw_page: RawPage = "testing...".parse().unwrap();
        assert_eq!(
            raw_page,
            RawPage {
                contents: "testing...".to_string(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn raw_page_from_str_empty_front_matter() {
        let contents = r#"
+++
+++
testing...
"#;
        let raw_page: RawPage = contents.parse().unwrap();
        assert_eq!(
            raw_page,
            RawPage {
                contents: "testing...\n".to_string(),
                ..Default::default()
            }
        );
    }

    #[test]
    fn raw_page_from_str_basic_front_matter() {
        let contents = r#"
+++
title = "Hello World!"
date = "2020-03-21"
+++
testing...
"#;
        let raw_page: RawPage = contents.parse().unwrap();
        assert_eq!(
            raw_page,
            RawPage {
                contents: "testing...\n".to_string(),
                front_matter: FrontMatter {
                    title: Some("Hello World!".to_string()),
                    date: Some(chrono::NaiveDate::from_ymd(2020, 3, 21)),
                    ..Default::default()
                }
            }
        );
    }

    #[test]
    fn raw_page_from_str_extra_front_matter() {
        let contents = r#"
+++
title = "Hello World!"
description = "My first post!"
date = "2020-03-21"
testing_int = 5
testing_str = "hello"
+++
testing...
"#;
        let raw_page: RawPage = contents.parse().unwrap();
        assert_eq!(
            raw_page,
            RawPage {
                contents: "testing...\n".to_string(),
                front_matter: FrontMatter {
                    title: Some("Hello World!".to_string()),
                    description: Some("My first post!".to_string()),
                    date: Some(chrono::NaiveDate::from_ymd(2020, 3, 21)),
                    kind: None,
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
        let temp_dir = tempfile::tempdir().unwrap();
        let root_dir = temp_dir.path().to_path_buf();
        fs::create_dir(root_dir.join("src")).unwrap();
        fs::write(root_dir.join("belong.toml"), "").unwrap();
        let project = Project::from_path(root_dir.clone()).unwrap();
        assert_eq!(
            project,
            Project {
                config: Config::new(root_dir.clone()),
                theme: Theme::from_path(&root_dir.join("theme")).unwrap(),
                pages: Vec::new(),
            }
        )
    }

    #[test]
    fn project_from_path_missing_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root_dir = temp_dir.path().to_path_buf();
        let err = Project::from_path(root_dir.clone()).unwrap_err();
        assert_eq!(
            format!("{:?}", err),
            format!(
                r#"failed to load config

Caused by:
    0: failed to load config file `{}`
    1: failed to read file
    2: {} (os error 2)"#,
                root_dir.join("belong.toml").display(),
                if cfg!(target_os = "windows") {
                    "The system cannot find the file specified."
                } else {
                    "No such file or directory"
                }
            )
        )
    }

    #[test]
    fn project_from_path_bad_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root_dir = temp_dir.path().to_path_buf();
        fs::write(root_dir.join("belong.toml"), "very bad toml").unwrap();
        let err = Project::from_path(root_dir.clone()).unwrap_err();
        assert_eq!(
            format!("{:?}", err),
            format!(
                r#"failed to load config

Caused by:
    0: failed to load config file `{}`
    1: failed to parse file contents
    2: expected an equals, found an identifier at line 1 column 6"#,
                root_dir.join("belong.toml").display()
            )
        );
    }

    #[test]
    fn project_from_path_bad_page() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root_dir = temp_dir.path().to_path_buf();
        fs::create_dir(root_dir.join("src")).unwrap();
        fs::write(root_dir.join("belong.toml"), "").unwrap();
        let page_content = r#"
+++
bad toml
+++
testing...
"#;
        let page_path = root_dir.join("src").join("test.md");
        fs::write(&page_path, &page_content).unwrap();
        let err = Project::from_path(root_dir.clone()).unwrap_err();
        assert_eq!(
            format!("{:?}", err),
            format!(
                r#"failed to load page `{}`

Caused by:
    0: failed to parse file contents
    1: failed to parse front matter
    2: expected an equals, found an identifier at line 2 column 5"#,
                page_path.display()
            )
        );
    }

    #[test]
    fn project_from_path_custom_config_pages_and_templates() {
        let temp_dir = tempfile::tempdir().unwrap();
        let root_dir = temp_dir.path().to_path_buf();
        let src_dir = root_dir.join("src");
        fs::create_dir(&src_dir).unwrap();
        let config_content = toml!(
            [project]
            title = "My Blog"

            [plugin]
            another = 5
        )
        .to_string();
        fs::write(root_dir.join("belong.toml"), &config_content).unwrap();
        let page_content = r#"
+++
title = "Hello World!"
date = "2020-03-2"
+++
testing...
"#;
        let page_path = root_dir.join("src").join("test.md");
        fs::write(&page_path, &page_content).unwrap();
        let project = Project::from_path(root_dir.clone()).unwrap();
        assert_eq!(
            project,
            Project {
                config: Config::from_path(root_dir.clone()).unwrap(),
                theme: Theme::from_path(&root_dir.join("theme")).unwrap(),
                pages: vec![Page::from_path(&src_dir, &page_path).unwrap()],
            }
        )
    }
}
