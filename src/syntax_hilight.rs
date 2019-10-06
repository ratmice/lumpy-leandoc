
use syntect::highlighting::{Theme, ThemeSet};
use syntect::dumps::{dump_to_file, from_dump_file};
use std::error::Error;
use std::path::Path;
/* allow until i have installer stuff figured out...
 * Until then make do with syntect built-in themes.
 */
#[allow(dead_code)]
pub fn load_theme(tm_file: &String, enable_caching: bool) -> Result<Theme, failure::Error> {
    let tm_path = Path::new(tm_file);

    if enable_caching {
        let tm_cache = tm_path.with_extension("tmdump");

        if tm_cache.exists() {
            match from_dump_file(tm_cache) {
                // From Result<T,dyn Error> to Result<T,errors::AppError>
                Err(e) => Err(e.into()),
                Ok(t) => Ok(t),
            }
        } else {
            let theme = ThemeSet::get_theme(tm_path)?;

            match dump_to_file(&theme, tm_cache) {
                Err(e) => {
                    println!("Warning: encountered error dumping theme cache (proceeding without cache): {}", e.description());
                    Ok(theme)
                }
                Ok(_) => Ok(theme),
            }
        }
    } else {
        Ok(ThemeSet::get_theme(tm_path)?)
    }
}

pub struct SyntaxCore {
    pub syntax_set: syntect::parsing::SyntaxSet,
    pub theme_set: syntect::highlighting::ThemeSet,
}

pub struct SyntaxStuff<'a> {
    pub core: &'a SyntaxCore,
    pub syntax: &'a syntect::parsing::SyntaxReference,
    pub theme: &'a syntect::highlighting::Theme,
}

pub const DEFAULT_THEME: &str = "Solarized (light)";

// ran vscode-lean's syntax highlighter through
// json -> plist -> https://github.com/aziz/SublimeSyntaxConvertor -> Lean3.sublime-syntax
// It would be nice to automate this.
pub const LEAN3_SYNTAX: &str = include_str!("../assets/Lean3.sublime-syntax");

pub fn setup_syntax_stuff() -> Result<SyntaxCore, failure::Error> {
    let mut ss_builder = syntect::parsing::SyntaxSetBuilder::new();
    let sd = syntect::parsing::syntax_definition::SyntaxDefinition::load_from_str(
        LEAN3_SYNTAX,
        true,
        Some("lean"),
    )?;
    ss_builder.add(sd);
    let syntax_set = ss_builder.build();
    let theme_set = ThemeSet::load_defaults();

    Ok(SyntaxCore {
        syntax_set: syntax_set,
        theme_set: theme_set,
    })
}

pub fn highlighter<'a>(
    lang: &str,
    theme_name: &str,
    cor: &'a SyntaxCore,
) -> Result<SyntaxStuff<'a>, failure::Error> {
    let theme: Option<&syntect::highlighting::Theme> = cor.theme_set.themes.get(theme_name);
    let lang_syntax = cor.syntax_set.find_syntax_by_extension(lang);
    match (lang_syntax, theme) {
        (Some(lang_syntax), Some(theme)) => Ok(SyntaxStuff {
            core: &cor,
            syntax: lang_syntax,
            theme: theme,
        }),
        (Some(_), None) => Err(format_err!("unable to load theme: '{}'", theme_name)),
        (None, Some(_)) => Err(format_err!("unable to load language syntax: '{}'", lang)),
        (None, None) => Err(format_err!(
            "unable to load language syntax for: '{}' or theme: '{}'",
            lang,
            theme_name
        )),
    }
}
