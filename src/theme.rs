//! Defines how we render a `Project`.

use std::borrow::Cow;
use std::ffi::OsString;
use std::fs;
use std::path;
use std::path::{Path, PathBuf};

use serde_json as json;
use serde_json::json;

use crate::app::Page;
use crate::config::Config;
use crate::output;
use crate::prelude::*;
use crate::renderer::Renderer;

/// Namespaced predefined templates.
mod template {
    pub const BASE: &str = include_str!("theme/templates/base.html");
    pub const INDEX: &str = include_str!("theme/templates/index.html");
    pub const PAGE: &str = include_str!("theme/templates/page.html");
}

/// Namespaced predefined stylesheets.
mod stylesheet {
    pub const CUSTOM: &str = include_str!("theme/css/custom.css");
}

/// A theme file.
type File = (PathBuf, Cow<'static, str>);

/////////////////////////////////////////////////////////////////////////
// Theme definitions
/////////////////////////////////////////////////////////////////////////

/// Represents an HTML template to use for rendering.
#[derive(Debug, PartialEq)]
struct Template {
    /// The name of the template.
    name: String,
    /// The template contents.
    contents: Cow<'static, str>,
}

/// Represents a CSS stylesheet to render.
#[derive(Debug, PartialEq)]
struct Stylesheet {
    /// The stylesheet path relative to the theme directory.
    path: PathBuf,
    /// The stylesheet contents.
    contents: Cow<'static, str>,
}

/// Represents the theme to use for rendering.
#[derive(Debug, PartialEq)]
pub struct Theme {
    /// Each of the theme's templates.
    templates: Vec<Template>,
    /// Each of the theme's stylesheets.
    stylesheets: Vec<Stylesheet>,
}

/////////////////////////////////////////////////////////////////////////
// Theme implementations
/////////////////////////////////////////////////////////////////////////

impl From<Stylesheet> for output::File {
    fn from(stylesheet: Stylesheet) -> Self {
        Self::new(stylesheet.path, stylesheet.contents)
    }
}

impl From<File> for Template {
    fn from((path, contents): File) -> Self {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        Self { name, contents }
    }
}

impl From<File> for Stylesheet {
    fn from((path, contents): File) -> Self {
        Self { path, contents }
    }
}

impl Page {
    /// Get the URL path for this page, relative to the root of the project.
    fn url_path(&self) -> Result<String> {
        self.path
            .with_extension("html")
            .components()
            .map(|c| c.as_os_str())
            .collect::<Vec<_>>()
            .join("/")
            .into_string()
            .map_err(|_| anyhow!("page path (and subsequently the URL) is not valid UTF-8"))
    }

    /// Naïve way of determining the path to the root of the project. This only
    /// works because `self.path()` is relative to the root of the project.
    fn url_path_to_root(&self) -> Result<String> {
        self.path
            .parent()
            .unwrap()
            .components()
            .fold(OsString::new(), |mut acc, c| match c {
                path::Component::Normal(_) => {
                    acc.push("../");
                    acc
                }
                _ => panic!("unexpected path component"),
            })
            .into_string()
            .map_err(|_| anyhow!("page path (and subsequently the URL) is not valid UTF-8"))
    }

    /// Rendering context for a `Page`.
    fn context(&self) -> Result<json::Value> {
        Ok(json!({
            "meta": self.front_matter,
            "path": self.url_path()?,
            "content": Renderer::new(&self.contents).render()
        }))
    }
}

impl Theme {
    fn load_theme_files_from_path<T>(
        theme_dir: &Path,
        sub_dir: &str,
        defaults: Vec<(&str, &'static str)>,
    ) -> Result<Vec<T>>
    where
        T: From<File>,
    {
        defaults
            .into_iter()
            .map(|(name, default)| {
                let relative_path: PathBuf = [sub_dir, name].iter().collect();
                let path = theme_dir.join(&relative_path);
                let contents = if path.exists() {
                    Cow::from(fs::read_to_string(&path).context("failed to read file")?)
                } else {
                    Cow::from(default)
                };
                Ok((relative_path, contents).into())
            })
            .collect()
    }

    /// Load a `Theme` from the given directory.
    ///
    /// If corresponding templates are present in the directory then they will
    /// override the default templates.
    pub fn from_path(theme_dir: &Path) -> Result<Self> {
        // Load the templates from disk, or set defaults.
        let templates = Self::load_theme_files_from_path(
            theme_dir,
            "templates",
            vec![
                ("base.html", template::BASE),
                ("index.html", template::INDEX),
                ("page.html", template::PAGE),
            ],
        )?;

        // Load the stylesheets from disk, or set defaults.
        let stylesheets = Self::load_theme_files_from_path(
            theme_dir,
            "css",
            vec![("custom.css", stylesheet::CUSTOM)],
        )?;

        Ok(Self {
            templates,
            stylesheets,
        })
    }

    /// Get a reference to the theme templates in the way that Tera wants.
    fn raw_templates(&self) -> Vec<(&str, &str)> {
        self.templates
            .iter()
            .map(|template| (template.name.as_str(), template.contents.as_ref()))
            .collect()
    }

    /// Render project pages using the given `Config`.
    pub fn render(self, config: Config, pages: Vec<Page>) -> Result<output::Output> {
        let mut output = output::Output::new(config);

        let mut templates = tera::Tera::default();
        templates
            .add_raw_templates(self.raw_templates())
            .context("failed to register templates")?;

        let mut base_ctx = tera::Context::new();
        base_ctx.insert("config", output.config().as_context());
        base_ctx.insert("path_to_root", "");

        let mut page_ctx = base_ctx.clone();
        let mut pages_ctx = Vec::new();

        for page in pages {
            let this_ctx = page.context().with_context(|| {
                format!(
                    "failed to generate render context for page `{}`",
                    page.path.display()
                )
            })?;
            page_ctx.insert("this", &this_ctx);
            page_ctx.insert("path_to_root", &page.url_path_to_root()?);
            pages_ctx.push(this_ctx);
            let rendered = templates
                .render("page.html", &page_ctx)
                .with_context(|| format!("failed to render page `{}`", page.path.display()))?;
            output.push_file(output::File::new(
                page.path.with_extension("html"),
                rendered,
            ));
        }

        base_ctx.insert("pages", &json::Value::Array(pages_ctx));
        let rendered = templates
            .render("index.html", &base_ctx)
            .context("failed to render page `index.html`")?;
        output.push_file(output::File::new("index.html".into(), rendered));

        for stylesheet in self.stylesheets {
            output.push_file(stylesheet.into());
        }

        Ok(output)
    }
}

/////////////////////////////////////////////////////////////////////////
// Unit tests
/////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_url_path_multi_dir() {
        let page = Page {
            path: ["path", "segment", "index.html"].iter().collect(),
            ..Default::default()
        };
        assert_eq!(page.url_path().unwrap(), "path/segment/index.html");
    }

    #[test]
    fn page_url_path_no_dir() {
        let page = Page {
            path: PathBuf::from("index.html"),
            ..Default::default()
        };
        assert_eq!(page.url_path().unwrap(), "index.html");
    }

    #[test]
    fn page_url_path_to_root_no_dir() {
        let page = Page {
            path: "index.html".into(),
            ..Default::default()
        };
        assert_eq!(page.url_path_to_root().unwrap(), "");
    }

    #[test]
    fn page_url_path_to_root_multi_dir() {
        let page = Page {
            path: ["path", "segment", "index.html"].iter().collect(),
            ..Default::default()
        };
        assert_eq!(page.url_path_to_root().unwrap(), "../../");
    }

    #[test]
    #[should_panic(expected = "unexpected path component")]
    fn page_url_path_to_root_unexpected_path_component() {
        let page = Page {
            path: ["/", "path", "segment"].iter().collect(),
            ..Default::default()
        };
        page.url_path_to_root().unwrap();
    }
}
