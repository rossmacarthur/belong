use lazy_static::lazy_static;
use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag};
use regex::Regex;

/// Fix a URL for HTML rendering.
///
/// For example `path/to/file.md#heading` becomes `path/to/file.html#heading`.
fn fix_markdown_url(url: CowStr) -> CowStr {
    lazy_static! {
        static ref MD_LINK: Regex = Regex::new(r"(?P<link>.*)\.md(?P<anchor>#.*)?").unwrap();
    }
    if let Some(captures) = MD_LINK.captures(&url) {
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

pub struct Renderer<'s> {
    parser: Parser<'s>,
}

impl<'s> Renderer<'s> {
    pub fn new(s: &'s str) -> Self {
        let parser = Parser::new_ext(s, Options::all());
        Self { parser }
    }

    pub fn render(self) -> String {
        let mut result = String::new();
        let events = self.parser.map(fix_markdown_links);
        html::push_html(&mut result, events);
        result
    }
}
