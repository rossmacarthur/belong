use std::fs;
use std::ops::{Bound, Range, RangeBounds, RangeFrom, RangeFull, RangeTo};
use std::path::{Path, PathBuf};

use regex::Captures;
use regex_macro::regex;

use crate::app::Page;
use crate::config::Config;
use crate::prelude::*;

/////////////////////////////////////////////////////////////////////////
// Definitions
/////////////////////////////////////////////////////////////////////////

/// A range of lines specified with some include directive.
#[derive(Debug, Clone, PartialEq)]
enum LineRange {
    Range(Range<usize>),
    RangeFrom(RangeFrom<usize>),
    RangeTo(RangeTo<usize>),
    RangeFull(RangeFull),
}

/// Represents an include preprocessing directive.
///
/// For example
///
/// ```markdown
/// {{ #include listing.rs:5:10 }}
/// ```
#[derive(Debug, Clone, PartialEq)]
struct Include {
    path: PathBuf,
    line_range: LineRange,
}

#[derive(Debug)]
enum DirectiveKind {
    Include(Include),
}

#[derive(Debug)]
struct Directive<'a> {
    kind: DirectiveKind,
    captures: Captures<'a>,
}

/////////////////////////////////////////////////////////////////////////
// Implementations
/////////////////////////////////////////////////////////////////////////

impl RangeBounds<usize> for LineRange {
    fn start_bound(&self) -> Bound<&usize> {
        match self {
            Self::Range(r) => r.start_bound(),
            Self::RangeFrom(r) => r.start_bound(),
            Self::RangeTo(r) => r.start_bound(),
            Self::RangeFull(r) => r.start_bound(),
        }
    }

    fn end_bound(&self) -> Bound<&usize> {
        match self {
            Self::Range(r) => r.end_bound(),
            Self::RangeFrom(r) => r.end_bound(),
            Self::RangeTo(r) => r.end_bound(),
            Self::RangeFull(r) => r.end_bound(),
        }
    }
}

impl LineRange {
    fn from_str(parts: Option<&str>) -> Result<Self> {
        let mut parts = parts.unwrap_or("").splitn(2, ':');
        let start = match parts.next().unwrap() {
            "" => None,
            // less 1 because line numbers start at 1.
            s => Some(
                s.parse::<usize>()
                    .context("failed to parse start line number")?
                    .saturating_sub(1),
            ),
        };
        let msg = "failed to parse end line number";
        Ok(match (start, parts.next()) {
            // no range given
            (None, None) => Self::RangeFull(..),
            // no colon after start, e.g. "5"
            (Some(start), None) => Self::Range(start..start + 1),
            // colon after start but no end, e.g. "5:"
            (Some(start), Some("")) => Self::RangeFrom(start..),
            // there is an end but no start, e.g. ":10"
            (None, Some(end)) => Self::RangeTo(..end.parse().context(msg)?),
            // there is start and end, e.g. "5:10"
            (Some(start), Some(end)) => Self::Range(start..end.parse().context(msg)?),
        })
    }

    fn start(&self) -> usize {
        use Bound::*;
        match self.start_bound() {
            Included(n) => *n,
            Excluded(n) => *n + 1,
            Unbounded => 0,
        }
    }

    fn end(&self) -> Option<usize> {
        use Bound::*;
        match self.end_bound() {
            Included(n) => Some(*n + 1),
            Excluded(n) => Some(*n),
            Unbounded => None,
        }
    }
}

impl Include {
    fn from_str(args: &str) -> Result<Self> {
        let mut parts = args.splitn(2, ':');
        let path = parts.next().unwrap().into();
        let line_range = LineRange::from_str(parts.next())?;
        Ok(Self { path, line_range })
    }

    fn extract(contents: String, line_range: LineRange) -> String {
        let start = line_range.start();
        let end = line_range.end();
        let lines = contents.lines().skip(start);
        match end {
            Some(end) => lines.take(end.saturating_sub(start)).collect::<Vec<_>>(),
            None => lines.collect::<Vec<_>>(),
        }
        .join("\n")
    }

    fn read(self, page_path: &Path) -> Result<String> {
        let Self { path, line_range } = self;
        let path = page_path.parent().unwrap().join(path);
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read from `{}`", path.display()))?;
        Ok(Self::extract(contents, line_range))
    }
}

impl Directive<'_> {
    fn range(&self) -> (usize, usize) {
        let m = self.captures.get(0).unwrap();
        (m.start(), m.end())
    }
}

fn find_directives(contents: &str) -> Result<Vec<Directive>> {
    let re = regex!(r"\{\{\s*#(?P<name>[a-zA-Z0-9_]+)\s+((?P<args>.*)\s*)\}\}");
    let mut directives = Vec::new();
    for captures in re.captures_iter(contents) {
        let name = captures.name("name").unwrap().as_str();
        let args = captures.name("args").unwrap().as_str();
        match name {
            "include" => match Include::from_str(args) {
                Ok(include) => {
                    let kind = DirectiveKind::Include(include);
                    directives.push(Directive { kind, captures })
                }
                err => log::warn!(
                    "{:?}\n",
                    err.with_context(|| format!(
                        "failed to parse include directive `{}`",
                        captures.get(0).unwrap().as_str()
                    ))
                    .unwrap_err()
                ),
            },
            name => log::warn!("unrecognized directive `{}`", name),
        };
    }
    Ok(directives)
}

fn preprocess(config: &Config, path: &Path, contents: &str) -> Result<String> {
    let mut new_contents = String::new();
    let mut previous_end = 0;
    for directive in find_directives(&contents)? {
        let (start, end) = directive.range();
        new_contents.push_str(&contents[previous_end..start]);
        match directive {
            Directive {
                kind: DirectiveKind::Include(include),
                ..
            } => {
                let page_path = config.src_dir().join(&path);
                new_contents.push_str(&include.read(&page_path)?);
            }
        }
        previous_end = end;
    }
    new_contents.push_str(&contents[previous_end..]);
    Ok(new_contents)
}

impl Page {
    /// Returns a preprocessed version of this `Page`.
    pub fn preprocess(self, config: &Config) -> Result<Self> {
        let Self {
            path,
            front_matter,
            contents,
        } = self;
        let contents = preprocess(config, &path, &contents)
            .with_context(|| format!("failed to preprocess page `{}`", path.display()))?;
        Ok(Self {
            path,
            front_matter,
            contents,
        })
    }
}

/////////////////////////////////////////////////////////////////////////
// Unit tests
/////////////////////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_range_from_str() -> Result<()> {
        assert_eq!(LineRange::from_str(None)?, LineRange::RangeFull(..));
        assert_eq!(LineRange::from_str(Some(""))?, LineRange::RangeFull(..));
        assert_eq!(LineRange::from_str(Some("0"))?, LineRange::Range(0..1));
        assert_eq!(LineRange::from_str(Some("1"))?, LineRange::Range(0..1));
        assert_eq!(LineRange::from_str(Some("5"))?, LineRange::Range(4..5));
        assert_eq!(LineRange::from_str(Some("5:"))?, LineRange::RangeFrom(4..));
        assert_eq!(LineRange::from_str(Some(":5"))?, LineRange::RangeTo(..5));
        assert_eq!(LineRange::from_str(Some("5:10"))?, LineRange::Range(4..10));
        Ok(())
    }

    #[test]
    fn include_from_str() -> Result<()> {
        assert_eq!(
            Include::from_str("listing.rs")?,
            Include {
                path: "listing.rs".into(),
                line_range: LineRange::RangeFull(..)
            }
        );
        assert_eq!(
            Include::from_str("listing.rs:")?,
            Include {
                path: "listing.rs".into(),
                line_range: LineRange::RangeFull(..)
            }
        );
        assert_eq!(
            Include::from_str("listing.rs:5:10")?,
            Include {
                path: "listing.rs".into(),
                line_range: LineRange::Range(4..10)
            }
        );
        Ok(())
    }

    #[test]
    fn include_extract() {
        assert_eq!(
            Include::extract("line 1\nline 2\nline 3".into(), LineRange::RangeFull(..)),
            "line 1\nline 2\nline 3",
        );
        assert_eq!(
            Include::extract("line 1\nline 2\nline 3".into(), LineRange::Range(0..1)),
            "line 1",
        );
        assert_eq!(
            Include::extract("line 1\nline 2\nline 3".into(), LineRange::RangeFrom(2..)),
            "line 3",
        );
        assert_eq!(
            Include::extract("line 1\nline 2\nline 3".into(), LineRange::RangeFrom(3..)),
            "",
        );
        assert_eq!(
            Include::extract("line 1\nline 2\nline 3".into(), LineRange::RangeTo(..0)),
            "",
        );
        assert_eq!(
            Include::extract("line 1\nline 2\nline 3".into(), LineRange::RangeTo(..2)),
            "line 1\nline 2",
        );
    }

    #[test]
    fn page_preprocess() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let root_dir = temp_dir.path().to_path_buf();
        fs::create_dir_all(root_dir.join("src"))?;
        let page_path = root_dir.join("src").join("page.md");
        let page_contents = r#"

```rust
{{#include ../listing.rs:3:5}}
```
"#;
        fs::write(&page_path, page_contents)?;
        fs::write(
            root_dir.join("listing.rs"),
            r#"

fn main() {
    println!("Hello World!");
}

"#,
        )?;

        let page = Page::from_path(&root_dir.join("src"), &page_path)?;
        assert_eq!(page.contents, page_contents);

        let page = page.preprocess(&Config::new(root_dir))?;
        assert_eq!(
            page.contents,
            r#"

```rust
fn main() {
    println!("Hello World!");
}
```
"#
        );

        Ok(())
    }
}
