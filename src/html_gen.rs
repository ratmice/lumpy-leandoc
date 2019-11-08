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
    // TODO
    // We probably shouldn't hard code "docs_style.css". Instead add a path to Lumpy.toml
    // copying that to the appropriate place. That would at least make it easy to install
    // the same stylesheet across multiple libraries.
    //
    // hard coded CSS classes:
    // * decl
    // * decl_par
    let md_result: Result<rope::Rope, failure::Error> =
        mods.iter().fold(Ok("".into()), |out, m| match &m {
            olean::types::Modification::Doc(name, contents) => {
                let parser = Parser::new_ext(contents, options);
                let parse_state = ParseState::new(parser, setup_syntax_stuff()?);
                let mut html_out = String::new();
                cmark::html::push_html(&mut html_out, parse_state);
                // This should be a /-! -/ doc_string */
                if name.to_string().is_empty() {
                    Ok(out? + html_out.into() + rope::Rope::from("<hr/>"))
                } else {
                    // and one for a declaration.
                    Ok(out?
                        + format!(
                            r#"<div class="decl"><h4>{}</h4><div class="decl_par">"#,
                            name
                        )
                        .into()
                        + html_out.into()
                        + "</div></div>".into())
                }
            }
            _ => out,
        });

    let header = format!(
        r#"<html><head><link rel="stylesheet" href="{}docs_style.css"></head>"#,
        // given a path like 'src/foo/bar/baz.lean' we want "../../"
        // FIXME This should be less terrible.
        // We probably don't need to worry about / vs \ path separator since it's output?
        path.as_ref()
            .iter()
            .skip(2)
            .fold(&mut String::new(), |acc, _| {
                acc.push_str("../");
                acc
            })
    );
    let md_result = md_result?;
    if !md_result.is_empty() {
        let _ = omap.insert(
            path,
            rope::Rope::from(header) + md_result + rope::Rope::from("</html>"),
        );
    }
    Ok(omap)
}

pub fn gen_html<'a>(
    acc: Result<im::ordmap::OrdMap<&'a Path, rope::Rope>, failure::Error>,
    olean: Result<&'a Path, errors::AppError>,
) -> Result<im::ordmap::OrdMap<&'a Path, rope::Rope>, failure::Error> {
    let elems = gen_elements(acc, olean?);
    elems
}
