use std::{borrow::Cow, fs, path::Path};

use anyhow::Context;

const BASE: &'static str = include_str!("theme/base.html");
const INDEX: &'static str = include_str!("theme/index.html");
const PAGE: &'static str = include_str!("theme/page.html");

/////////////////////////////////////////////////////////////////////////
// Theme definitions
/////////////////////////////////////////////////////////////////////////

/// Represents an HTML template to use for rendering.
#[derive(Debug, PartialEq)]
pub struct Template {
    contents: Cow<'static, str>,
}

/// Represents the theme to use for rendering.
#[derive(Debug, Default, PartialEq)]
pub struct Theme {
    /// Each of the theme's templates.
    templates: Vec<(&'static str, Template)>,
}

/////////////////////////////////////////////////////////////////////////
// Theme implementations
/////////////////////////////////////////////////////////////////////////

impl From<&'static str> for Template {
    fn from(s: &'static str) -> Self {
        Self {
            contents: Cow::from(s),
        }
    }
}

impl From<String> for Template {
    fn from(s: String) -> Self {
        Self {
            contents: Cow::from(s),
        }
    }
}

impl Template {
    pub fn contents(&self) -> &str {
        &self.contents
    }
}

impl Theme {
    /// Load a `Theme` from the given directory.
    pub fn from_path(theme_dir: &Path) -> anyhow::Result<Self> {
        let defaults = &[
            ("base.html", BASE),
            ("index.html", INDEX),
            ("page.html", PAGE),
        ];
        let mut templates = Vec::with_capacity(defaults.len());
        for (name, default) in defaults {
            let path = theme_dir.join(name);
            let template = if path.exists() {
                Template::from(fs::read_to_string(&path).context("failed to read file")?)
            } else {
                Template::from(*default)
            };
            templates.push((*name, template));
        }
        Ok(Self { templates })
    }

    pub fn templates(&self) -> Vec<(&'static str, &str)> {
        self.templates
            .iter()
            .map(|(name, template)| (*name, template.contents()))
            .collect()
    }
}
