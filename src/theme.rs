use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;

mod template {
    pub const BASE: &str = include_str!("theme/base.html");
    pub const INDEX: &str = include_str!("theme/index.html");
    pub const PAGE: &str = include_str!("theme/page.html");
}

mod stylesheet {
    pub const CUSTOM: &str = include_str!("theme/css/custom.css");
}

/////////////////////////////////////////////////////////////////////////
// Theme definitions
/////////////////////////////////////////////////////////////////////////

/// Represents an HTML template to use for rendering.
#[derive(Debug, PartialEq)]
pub struct Template {
    name: &'static str,
    contents: Cow<'static, str>,
}

/// Represents a CSS stylesheet to render.
#[derive(Debug, PartialEq)]
pub struct Stylesheet {
    path: PathBuf,
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

impl Template {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }
}

impl Stylesheet {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }
}

impl Theme {
    /// Load a `Theme` from the given directory.
    pub fn from_path(theme_dir: &Path) -> anyhow::Result<Self> {
        // Load the templates from disk, or set defaults.
        let defaults = vec![
            ("base.html", template::BASE),
            ("index.html", template::INDEX),
            ("page.html", template::PAGE),
        ];
        let mut templates = Vec::with_capacity(defaults.len());

        for (name, default) in defaults.into_iter() {
            let path = theme_dir.join(name);
            let contents = if path.exists() {
                Cow::from(fs::read_to_string(&path).context("failed to read file")?)
            } else {
                Cow::from(default)
            };
            templates.push(Template { name, contents });
        }

        // Load the stylesheets from disk, or set defaults.
        let defaults = vec![(PathBuf::from("css/custom.css"), stylesheet::CUSTOM)];
        let mut stylesheets = Vec::with_capacity(defaults.len());

        for (relative_path, default) in defaults.into_iter() {
            let path = theme_dir.join(&relative_path);
            let contents = if path.exists() {
                Cow::from(fs::read_to_string(&path).context("failed to read file")?)
            } else {
                Cow::from(default)
            };
            stylesheets.push(Stylesheet {
                path: relative_path,
                contents,
            })
        }

        Ok(Self {
            templates,
            stylesheets,
        })
    }

    pub fn raw_templates(&self) -> Vec<(&str, &str)> {
        self.templates
            .iter()
            .map(|template| (template.name(), template.contents()))
            .collect()
    }

    pub fn stylesheets(&self) -> &[Stylesheet] {
        &self.stylesheets
    }
}
