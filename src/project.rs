use std::{
    ffi::OsString,
    fs,
    io::Write,
    path::{self, Path, PathBuf},
    str,
};

use anyhow::Context;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    config::Config,
    render::Renderer,
    theme::Theme,
    util::{self, FromPath, Join, TomlValueExt},
};

/////////////////////////////////////////////////////////////////////////
// Project definitions
/////////////////////////////////////////////////////////////////////////

/// Represents the TOML front matter of a Markdown document.
#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct FrontMatter {
    /// The title for this page.
    title: Option<String>,
    /// The description for this page.
    description: Option<String>,
    /// The date this page was written.
    date: Option<chrono::NaiveDate>,
    /// The type of page this is.
    #[serde(rename = "type")]
    kind: Option<String>,
    /// The rest of the TOML front matter.
    #[serde(flatten)]
    rest: toml::Value,
}

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
    path: PathBuf,
    /// Front matter for the page.
    front_matter: FrontMatter,
    /// The contents of the page.
    contents: String,
}

/// A builder for `Project`.
#[derive(Debug)]
pub struct Builder {
    root_dir: PathBuf,
    gitignore: bool,
}

/// Represents our entire project.
#[derive(Debug, PartialEq)]
pub struct Project {
    /// The project's root directory.
    root_dir: PathBuf,
    /// The configuration used to control how the project is built.
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

impl str::FromStr for RawPage {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: regex::Regex =
                regex::Regex::new(r"^\s*\+\+\+((?s).*(?-s))\+\+\+(\r?\n)+((?s).*(?-s))$").unwrap();
        }
        let mut contents = s;
        let front_matter = match RE.captures(contents) {
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
    fn from_path(src_dir: &Path, full_path: &Path) -> anyhow::Result<Self> {
        let raw_page = RawPage::from_path(&full_path)?;
        let path = full_path.strip_prefix(&src_dir).unwrap().to_path_buf();
        Ok(Self {
            path,
            front_matter: raw_page.front_matter,
            contents: raw_page.contents,
        })
    }

    /// Get the URL path for this page, relative to the root of the project.
    fn url_path(&self) -> OsString {
        self.path
            .with_extension("html")
            .components()
            .map(|c| c.as_os_str())
            .collect::<Vec<_>>()
            .join("/")
    }

    /// Naïve way of determining the path to the root of the project. This only
    /// works because `self.path` is relative to the root of the project.
    fn url_path_to_root(&self) -> OsString {
        self.path
            .parent()
            .unwrap()
            .components()
            .fold(OsString::new(), |mut acc, c| {
                if let path::Component::Normal(_) = c {
                    acc.push("../");
                } else {
                    panic!("unexpected path component");
                }
                acc
            })
    }

    /// Rendering context for a `Page`.
    fn context(&self) -> serde_json::Value {
        json!({
            "meta": self.front_matter,
            "path": self.url_path(),
            "content": Renderer::new(&self.contents).render()
        })
    }
}

impl Builder {
    /// Create a new `Builder`.
    pub fn new<P>(root_dir: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let root_dir = root_dir.into();
        Self {
            root_dir,
            gitignore: true,
        }
    }

    /// Whether to create a `.gitignore` file.
    pub fn gitignore(mut self, gitignore: bool) -> Self {
        self.gitignore = gitignore;
        self
    }

    /// Build the `Project` and return the built project.
    pub fn build(self) -> anyhow::Result<()> {
        // Create directory structure.
        let src_dir = self.root_dir.join("src");
        fs::create_dir_all(&src_dir)
            .with_context(|| format!("failed to create src directory `{}`", src_dir.display()))?;

        // Create .gitignore file.
        if self.gitignore {
            let gitignore_file = self.root_dir.join(".gitignore");
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&gitignore_file)
                .with_context(|| {
                    format!(
                        "failed to create gitignore file `{}`",
                        gitignore_file.display()
                    )
                })?;
            writeln!(file, "public").context("failed to write to gitignore file")?;
        }

        // Create config file.
        let config_file = self.root_dir.join("belong.toml");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&config_file)
            .with_context(|| format!("failed to create config file `{}`", config_file.display()))?;
        file.write(&toml::to_vec(&Config::default()).unwrap())
            .context("failed to write config to config file")?;

        Ok(())
    }
}

impl Project {
    /// Load a `Project` from the given directory.
    pub fn from_path<P>(root_dir: P) -> anyhow::Result<Self>
    where
        P: Into<PathBuf>,
    {
        let root_dir = root_dir.into();

        // Load the config file from disk.
        let config_file = root_dir.join("belong.toml");
        let config = Config::from_path(&config_file)
            .with_context(|| format!("failed to load config `{}`", &config_file.display()))?;

        // Load theme theme from disk.
        let theme_dir = root_dir.join("theme");
        let theme = Theme::from_path(&theme_dir).context("failed to load theme")?;

        // Finally load all the the pages from disk.
        let src_dir = root_dir.join("src");
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
            root_dir,
            config,
            theme,
            pages,
        })
    }

    pub fn output_dir(&self) -> PathBuf {
        self.root_dir.join("public")
    }

    pub fn render(&self) -> anyhow::Result<()> {
        let output_dir = self.output_dir();
        util::recreate_dir(&output_dir).with_context(|| {
            format!(
                "failed to recreate output directory `{}`",
                output_dir.display()
            )
        })?;

        let mut templates = tera::Tera::default();
        templates
            .add_raw_templates(self.theme.raw_templates())
            .context("failed to register templates")?;

        let mut base_ctx = tera::Context::new();
        base_ctx.insert("config", &self.config);
        base_ctx.insert("path_to_root", "");

        let mut page_ctx = base_ctx.clone();
        let mut pages_ctx = Vec::new();

        for page in &self.pages {
            let this_ctx = page.context();
            page_ctx.insert("this", &this_ctx);
            page_ctx.insert("path_to_root", &page.url_path_to_root());

            pages_ctx.push(this_ctx);

            // Render page
            let rendered = templates
                .render("page.html", &page_ctx)
                .with_context(|| format!("failed to render page `{}`", page.path.display()))?;
            // Write page to file
            let dst = output_dir.join(&page.path);
            let dir = dst.parent().unwrap();
            fs::create_dir_all(dir)
                .with_context(|| format!("failed to create directory `{}`", dir.display()))?;
            fs::write(&dst, rendered)
                .with_context(|| format!("failed to write page `{}`", dst.display()))?;
        }

        base_ctx.insert("pages", &serde_json::Value::Array(pages_ctx));

        // Render page
        let rendered = templates
            .render("index.html", &base_ctx)
            .context("failed to render page `index.html`")?;
        // Write page to file
        fs::write(output_dir.join("index.html"), rendered)
            .context("failed to write page `index.html`")?;

        for stylesheet in self.theme.stylesheets() {
            // Write stylesheet to file
            let dst = output_dir.join(stylesheet.path());
            let dir = dst.parent().unwrap();
            fs::create_dir_all(dir)
                .with_context(|| format!("failed to create directory `{}`", dir.display()))?;
            fs::write(&dst, stylesheet.contents())
                .with_context(|| format!("failed to render stylesheet `{}`", dst.display()))?;
        }

        Ok(())
    }
}

/////////////////////////////////////////////////////////////////////////
// Unit tests
/////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod test {
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
    fn page_url_path_multi_dir() {
        let page = Page {
            path: ["path", "segment", "index.html"].iter().collect(),
            ..Default::default()
        };
        assert_eq!(page.url_path(), OsString::from("path/segment/index.html"));
    }

    #[test]
    fn page_url_path_no_dir() {
        let page = Page {
            path: PathBuf::from("index.html"),
            ..Default::default()
        };
        assert_eq!(page.url_path(), OsString::from("index.html"));
    }

    #[test]
    fn page_url_path_to_root_no_dir() {
        let page = Page {
            path: "index.html".into(),
            ..Default::default()
        };
        assert_eq!(page.url_path_to_root(), OsString::from(""));
    }

    #[test]
    fn page_url_path_to_root_multi_dir() {
        let page = Page {
            path: ["path", "segment", "index.html"].iter().collect(),
            ..Default::default()
        };
        assert_eq!(page.url_path_to_root(), OsString::from("../../"));
    }

    #[test]
    #[should_panic(expected = "unexpected path component")]
    fn page_url_path_to_root_unexpected_path_component() {
        let page = Page {
            path: ["/", "path", "segment"].iter().collect(),
            ..Default::default()
        };
        page.url_path_to_root();
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
                root_dir: root_dir.clone(),
                config: Config::default(),
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
                r#"failed to load config `{}`

Caused by:
    0: failed to read file
    1: {} (os error 2)"#,
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
                r#"failed to load config `{}`

Caused by:
    0: failed to parse file contents
    1: expected an equals, found an identifier at line 1 column 6"#,
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
                root_dir: root_dir.clone(),
                config: str::FromStr::from_str(&config_content).unwrap(),
                theme: Theme::from_path(&root_dir.join("theme")).unwrap(),
                pages: vec![Page::from_path(&src_dir, &page_path).unwrap()],
            }
        )
    }
}
