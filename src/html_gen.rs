use pulldown_cmark::{Parser, html};
use pulldown_cmark as cmark;
use crate::errors;
use std::fs::File;
use crate::syntax_hilight::*;
use std::path::Path;

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
    let syntax_core = setup_syntax_stuff()?;

    let md_result: Result<(rope::Rope, _, _), failure::Error> =
        mods.iter()
            .fold(Ok(("".into(), &syntax_core, None)), |result, m| match &m {
                olean::types::Modification::Doc(_name, contents) => {
                    let parser = Parser::new_ext(contents, options);
                    let mut html_output = String::new();
                    html::push_html(&mut html_output, parser);
                    let umm : Option<String> = None;
                    Ok((result?.0 + html_output.into(), &syntax_core, umm))
                }
                _ => {
                    let (rope, syntax_core, _) = result?;
                    Ok((rope, &syntax_core, None))
                }
            });
    let _ = omap.insert(path, md_result?.0);
    Ok(omap)
}

pub fn gen_html<'a>(
    acc: Result<im::ordmap::OrdMap<&'a Path, rope::Rope>, failure::Error>,
    olean: Result<&'a Path, errors::AppError>,
) -> Result<im::ordmap::OrdMap<&'a Path, rope::Rope>, failure::Error> {
    let elems = gen_elements(acc, olean?);
    elems
}
