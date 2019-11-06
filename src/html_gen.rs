use crate::errors;
use crate::syntax_hilight::*;
use pulldown_cmark as cmark;
use pulldown_cmark::{Event, Parser, Tag};
use std::fs::File;
use std::path::Path;
use syntect as synt;
struct ParseState<'a> {
    p: Parser<'a>,
    sc: SyntaxCore,
    theme_name: String,
    lang: Option<String>,
}

impl<'a> ParseState<'a> {
    pub fn new(p: Parser<'a>, sc: SyntaxCore) -> Self {
        ParseState {
            p,
            sc: sc,
            lang: None,
            theme_name: DEFAULT_THEME.to_string(),
        }
    }
}

impl<'a> Iterator for ParseState<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.p.next().map(|e: Self::Item| match e {
            Event::Start(Tag::CodeBlock(lang)) => {
                self.lang = Some(lang.to_string());
                Event::Start(Tag::CodeBlock(lang))
            }

            Event::End(Tag::CodeBlock(lang)) => {
                self.lang = None;
                Event::End(Tag::CodeBlock(lang))
            }

            Event::Text(text) => match &self.lang {
                Some(lang) => highlighter(&lang, &self.theme_name, &self.sc)
                    .map(|stuff| {
                        Event::Html(
                            synt::html::highlighted_html_for_string(
                                &text,
                                &self.sc.syntax_set,
                                &stuff.syntax,
                                &stuff.theme,
                            )
                            .into(),
                        )
                    })
                    .unwrap_or(Event::Text(text)),
                None => Event::Text(text),
            },
            Event::Code(text) => {
                let lang = match &self.lang {
                    Some(l) => l,
                    None => "lean",
                };
                highlighter(&lang, &self.theme_name, &self.sc)
                    .map(|stuff| {
                        let mut h = synt::easy::HighlightLines::new(stuff.syntax, &stuff.theme);
                        let regions = h.highlight(&text, &stuff.core.syntax_set);
                        let html = synt::html::styled_line_to_highlighted_html(
                            &regions[..],
                            synt::html::IncludeBackground::Yes,
                        );
                        Event::Html(html.into()).into()
                    })
                    .unwrap_or(Event::Text(text))
            }

            e => e,
        })
    }
}

pub fn gen_elements<P: AsRef<Path>>(
    acc_r: Result<im::ordmap::OrdMap<P, rope::Rope>, failure::Error>,
    path: P,
) -> Result<im::ordmap::OrdMap<P, rope::Rope>, failure::Error>
where
    P: std::cmp::Ord + std::clone::Clone,
{
    let mut omap: im::ordmap::OrdMap<P, rope::Rope> = acc_r?;
    let ol = olean_rs::deserialize::read_olean(File::open(&path)?)?;
    let mods = olean_rs::deserialize::read_olean_modifications(&ol.code)?;
    let options = cmark::Options::empty();
    let md_result: Result<rope::Rope, failure::Error> =
        mods.iter().fold(Ok("<html><head><style>.indent{ padding-left: 1em; padding-right: 1em;}</style></head>".into()), |out, m| match &m {
            olean::types::Modification::Doc(_name, contents) => {
                let parser = Parser::new_ext(contents, options);
                let parse_state = ParseState::new(parser, setup_syntax_stuff()?);
                let mut html_out = String::new();
                cmark::html::push_html(&mut html_out, parse_state);
                if _name.to_string().is_empty() { Ok(out? + html_out.into() + rope::Rope::from("<hr/>")) } // This should be a /-! -/ doc_string */
                else { Ok(out? + format!(r#"<div><h4>{}</h4><div class="indent">"#, _name).into() +  html_out.into() + "</div></div>".into()) } // and one for a declaration _name.
            }
            _ => out,
        });
    let _ = omap.insert(path, md_result? + rope::Rope::from("</html>"));
    Ok(omap)
}

pub fn gen_html<'a>(
    acc: Result<im::ordmap::OrdMap<&'a Path, rope::Rope>, failure::Error>,
    olean: Result<&'a Path, errors::AppError>,
) -> Result<im::ordmap::OrdMap<&'a Path, rope::Rope>, failure::Error> {
    let elems = gen_elements(acc, olean?);
    elems
}
