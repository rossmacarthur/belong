//! Markdown to HTML renderer.
//!
//! This wraps [`pulldown_cmark::Parser`] and
//! [`pulldown_cmark::html::push_html`] and applies some special fixes when
//! rendering to HTML.
//!
//! [`pulldown_cmark::Parser`]: ../../pulldown_cmark/struct.Parser.html
//! [`pulldown_cmark::html::push_html`]:
//! ../../pulldown_cmark/html/fn.push_html.html

use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag};
use regex_macro::regex;

/// Fix a URL for HTML rendering.
///
/// For example `path/to/file.md#heading` becomes `path/to/file.html#heading`.
fn fix_markdown_url(url: CowStr) -> CowStr {
    let re = regex!(r"(?P<link>.*)\.md(?P<anchor>#.*)?");
    if let Some(captures) = re.captures(&url) {
        CowStr::from(format!(
            "{link}.html{anchor}",
            link = &captures["link"],
            anchor = captures.name("anchor").map(|m| m.as_str()).unwrap_or("")
        ))
    } else {
        url
    }
}

/// Fix Markdown links by replacing `.md` with `.html`.
fn fix_markdown_links(event: Event) -> Event {
    match event {
        Event::Start(Tag::Link(link_type, url, title)) => {
            Event::Start(Tag::Link(link_type, fix_markdown_url(url), title))
        }
        Event::Start(Tag::Image(link_type, url, title)) => {
            Event::Start(Tag::Image(link_type, fix_markdown_url(url), title))
        }
        _ => event,
    }
}

/// A Markdown to HTML renderer.
pub struct Renderer<'s> {
    /// The raw parser.
    parser: Parser<'s>,
}

impl<'s> Renderer<'s> {
    /// Create a new `Renderer`.
    pub fn new(s: &'s str) -> Self {
        let parser = Parser::new_ext(s, Options::all());
        Self { parser }
    }

    /// Consume the `Renderer` and output HTML.
    pub fn render(self) -> String {
        let mut result = String::new();
        let events = self.parser.map(fix_markdown_links);
        html::push_html(&mut result, events);
        result
    }
}
