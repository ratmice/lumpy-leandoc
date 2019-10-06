extern crate crowbook_text_processing;
extern crate im;
extern crate syntect;
use crate::errors;
use crate::syntax_hilight::{SyntaxStuff, SyntaxCore, highlighter, setup_syntax_stuff, DEFAULT_THEME};
use crowbook_text_processing::escape;
use pulldown_cmark as cmark;
use std::fs::File;
use std::ops::Range;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::util::LinesWithEndings;
use syntect::highlighting::{Color, Style};

pub fn stylish_tex(r: rope::Rope, tuple: (Style, &str)) -> rope::Rope {
    let (style, text) = tuple;
    let fg = style.foreground;
    let fs = style.font_style;
    let underline = fs.contains(syntect::highlighting::FontStyle::UNDERLINE);
    let bold = fs.contains(syntect::highlighting::FontStyle::BOLD);
    let italic = fs.contains(syntect::highlighting::FontStyle::ITALIC);

    r + if underline {
        r"\underline{".into()
    } else {
        "{".into()
    } + if bold { r"\bold{".into() } else { "{".into() }
        + if italic {
            r"\italic{".into()
        } else {
            "{".into()
        }
        + format!(
            "\\color[RGB]{{{}, {}, {}}} {} ",
            fg.r,
            fg.g,
            fg.b,
            escape::tex(text.to_string())
        )
        .into()
        + "}}}".into()
}



pub fn lit<'a>(stuff: &'a SyntaxStuff, text: String, inline: bool) -> rope::Rope {
    let mut highlighter = HighlightLines::new(stuff.syntax, &stuff.theme);
    LinesWithEndings::from(&text).fold("".into(), |folding, line| {
        let ranges: Vec<(Style, &str)> = highlighter.highlight(line, &stuff.core.syntax_set);
        let highlighted = folding
            + ranges
                .iter()
                .fold("".into(), |folding, v| stylish_tex(folding, *v));
        if inline {
            highlighted
        } else {
            highlighted + "\\\\\n".into()
        }
    })
}

pub fn handle_event<'a>(
    tuple: Result<(rope::Rope, &'a SyntaxCore, Option<String>), failure::Error>,
    event_tuple: (pulldown_cmark::Event<'a>, Range<usize>),
) -> Result<(rope::Rope, &'a SyntaxCore, Option<String>), failure::Error> {
    let (rope, syntax_core, parse_state) = tuple?;
    match event_tuple.0 {
        cmark::Event::Text(text) => {
            let foo = parse_state.as_ref().map(String::as_str);
            match foo {
                None => Ok((
                    rope + escape::tex(text.to_string()).into() + "\n".into(),
                    syntax_core,
                    None,
                )),
                Some("latex") => Ok((rope + text.into(), syntax_core, parse_state)),
                l => {
                    let lang = match l {
                        None => "lean", // default for a ```foo```-like inline code block.
                        Some("") => "lean",
                        Some(lang) => lang,
                    };
                    let syntax_stuff = highlighter(lang, DEFAULT_THEME, syntax_core)?;
                    let highlighted_code = lit(&syntax_stuff, text.to_string(), false); // false because it comes from a code fence.
                    Ok((
                        rope + highlighted_code.into() + "\n".into(),
                        syntax_core,
                        parse_state,
                    ))
                }
            }
        }
        cmark::Event::Code(text) => {
            // In particular this should always match None,
            // unless some markdown extension is in play
            // as the cmark::Event::Start(CodeBlock(lang)) sets the parse state, then should route
            // through Text above.
            //
            // Here we set the language for code blocks with no specified language.
            // This should be trimming whitespace at some point
            let lang = match parse_state.as_ref().map(String::as_str) {
                None => "lean",     // default for a ```foo```-like inline code block.
                Some(lang) => lang, // Perhaps in the future this case matters.
            };
            let stuff = highlighter(lang, DEFAULT_THEME, syntax_core)?;
            let c = stuff.theme.settings.background.unwrap_or(Color::WHITE);
            let cbox = rope::Rope::from(format!(r"\colorbox[RGB]{{{},{},{}}}{{", c.r, c.g, c.b));
            let syntax_stuff = highlighter(&lang, DEFAULT_THEME, syntax_core)?;
            let highlighted_code = lit(&syntax_stuff, text.to_string(), true); // true, inline code block.
            Ok((
                rope + cbox + highlighted_code.into() + "}".into(),
                syntax_core,
                None,
            ))
        }
        cmark::Event::Start(cmark::Tag::CodeBlock(lang)) => {
            // This should be trimming whitespace at some point
            let lang = match lang.as_ref() {
                // I suppose we hit here: ```\ncode...```
                "" => "lean",
                l => l, // Perhaps in the future this case matters.
            };

            match lang.as_ref() {
                "latex" => Ok((rope + "{{".into(), syntax_core, Some(lang.to_string()))),
                _ => {
                    let stuff = highlighter(&lang, DEFAULT_THEME, syntax_core)?;
                    let c = stuff.theme.settings.background.unwrap_or(Color::WHITE);
                    let block_begin = rope::Rope::from(format!(
                        "\\\\\n\\colorbox[RGB]{{{},{},{}}}{{\\parbox{{4.5in}}{{",
                        c.r, c.g, c.b
                    ));

                    Ok((rope + block_begin, syntax_core, Some(lang.to_string())))
                }
            }
        }
        // I guess we shall see if there are nested code blocks -- it doesn't seem sane i.e.
        // ```lean ```latex... ``` ``` where the end of ```latex should return the parse state
        // containing lean. and we're trying to actually parse the latex markdown.
        // I'm not going to implement anything of the sort unless there is some justifiable
        // reason to do so.
        // If so, this should pop a stack rather than return none.
        cmark::Event::End(cmark::Tag::CodeBlock(_)) => Ok((rope + r"}}".into(), syntax_core, None)),
        cmark::Event::HardBreak => Ok((rope + r"\linebreak".into(), syntax_core, None)),
        cmark::Event::SoftBreak => Ok((rope, syntax_core, None)),
        cmark::Event::Start(cmark::Tag::Emphasis) => {
            Ok((rope + r"\emph{".into(), syntax_core, None))
        }
        cmark::Event::End(cmark::Tag::Emphasis) => Ok((rope + r"}".into(), syntax_core, None)),
        cmark::Event::Start(cmark::Tag::List(Some(ord_first))) => Ok((
            rope + format!("\\begin{{enumerate}}[{}]\n", ord_first).into(),
            syntax_core,
            None,
        )),
        cmark::Event::End(cmark::Tag::List(Some(_))) => {
            Ok((rope + r"\end{enumerate}".into(), syntax_core, None))
        }
        cmark::Event::Start(cmark::Tag::List(None)) => {
            Ok((rope + r"\begin{itemize}".into(), syntax_core, None))
        }
        cmark::Event::End(cmark::Tag::List(None)) => {
            Ok((rope + r"\end{itemize}".into(), syntax_core, None))
        }
        cmark::Event::Start(cmark::Tag::Item) => Ok((rope + r"\item ".into(), syntax_core, None)),
        cmark::Event::End(cmark::Tag::Item) => Ok((rope + "\n".into(), syntax_core, None)),
        cmark::Event::Start(cmark::Tag::Paragraph) => {
            Ok((rope + "\\par\n".into(), syntax_core, None))
        }
        cmark::Event::End(cmark::Tag::Paragraph) => Ok((rope, syntax_core, None)),
        cmark::Event::Start(cmark::Tag::Link(_, url, title)) => Ok((
            rope + format!("\\href{{{}}}{{{}}}", url, title).into(),
            syntax_core,
            None,
        )),
        cmark::Event::End(cmark::Tag::Link(_, _, _)) => Ok((rope, syntax_core, None)),
        cmark::Event::Html(html_str) => Err(errors::ErrorKind::MarkdownLatexConversion(format!(
            "unsupported html: {}",
            html_str
        ))
        .into()),
        cmark::Event::InlineHtml(html_str) => Err(errors::AppError::App(
            errors::ErrorKind::MarkdownLatexConversion(format!("unsupported inline html: {}", html_str)),
        )
        .into()),
        cmark::Event::FootnoteReference(s) => Ok((
            rope + format!("\\footref{{{}}}%Does it work?\n", s).into(),
            syntax_core,
            None,
        )),
        cmark::Event::TaskListMarker(_flag) => Err(errors::ErrorKind::MarkdownLatexConversion(
            "unsupported task list marker:".to_string(),
        )
        .into()),
        cmark::Event::Start(cmark::Tag::Rule) => {
            Ok((rope + r"\hrulefill ".into(), syntax_core, None))
        }
        cmark::Event::End(cmark::Tag::Rule) => Ok((rope, syntax_core, None)),
        _ => Err(errors::ErrorKind::MarkdownLatexConversion(format!("unsupported markdown tag")).into()),
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
    let syntax_core = setup_syntax_stuff()?;

    let md_result: Result<(rope::Rope, _, _), failure::Error> =
        mods.iter()
            .fold(Ok(("".into(), &syntax_core, None)), |result, m| match &m {
                olean::types::Modification::Doc(name, contents) => {
                    let (rope, syntax_core, parse_state) = result?;
                    let rope =
                        rope + format!("\\paragraph{{{}}}\n", escape::tex(name.to_string())).into();
                    let parser = cmark::Parser::new_ext(contents.as_str(), options);
                    parser
                        .into_offset_iter()
                        .fold(Ok((rope, &syntax_core, parse_state)), |result, event| {
                            handle_event(result, event)
                        })
                }
                _ => {
                    let (rope, syntax_core, _) = result?;
                    Ok((rope, &syntax_core, None))
                }
            });
    let _ = omap.insert(path, md_result?.0);
    Ok(omap)
}

pub fn gen_latex<'a>(
    acc: Result<im::ordmap::OrdMap<&'a Path, rope::Rope>, failure::Error>,
    olean: Result<&'a Path, errors::AppError>,
) -> Result<im::ordmap::OrdMap<&'a Path, rope::Rope>, failure::Error> {
    let elems = gen_elements(acc, olean?);
    elems
}

fn packages() -> rope::Rope {
    let pkgs = vec![
        "microtype",
        "hyperref",
        "fontspec",
        "xcolor",
        "newunicodechar",
        "bussproofs",
        "listings",
        "titlesec",
        "parskip",
        "enumerate",
        "contour",
        "ulem",
    ];

    pkgs.iter().fold(rope::Rope::from(""), |ret, pkg| {
        ret + rope::Rope::from(format!("\\usepackage{{{}}}\n", pkg))
    })
}

fn newunichar(it: char, fallback: &str) -> String {
    let it = it.to_string();
    let fallback = fallback.to_string();
    format!(
        "\\AtBeginDocument{{\\newunicodechar{{{}}} {{{}}}}}\n",
        it, fallback
    )
}

fn unicode_hack() -> rope::Rope {
    // FIXME
    const _SUP_SMALL_LATIN: &str = "ᵃᵇᶜᵈᵉᶠᵍʰⁱʲᵏˡᵐⁿᵒᵖʳˢᵗᵘᵛʷˣʸᶻ";
    const _SUP_CAP_LATIN: &str = "ᴬᴮᴰᴱᴳᴴᴵᴶᴷᴸᴹᴺᴼᴾᴿᵀᵁⱽᵂ";
    const _SUB_LATIN: &str = "ₐₑₕᵢⱼₖₗₘₙₒₚᵣₛₜᵤᵥₓ";
    const _SUP_GREEK: &str = "ᵝᵞᵟᵋᶿᶥᵠᵡ";
    const _SUB_GREEK: &str = "ᵦᵧᵨᵩᵪ";
    const _SUP_NUMSYM: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁼⁽⁾";
    const _SUB_NUMSYM: &str = "₀₁₂₃₄₅₆₇₈₉₊₋₌₍₎";

    /* well, this is kind of an ugly expression */
    "←→↓↔↦↪∀∃∅∈∉∘√∞∣∥∧∨∩∪≃≅≠≡≤≥≫≺≼⊂⊆⊑⊓⊔⊕⊢⊤⊥⋁⋀⋃⋂⋙⋯⌊⌋⟨⟩⟶⥤"
        .chars()
        .fold(rope::Rope::from(""), |ret, c| {
            ret + rope::Rope::from(newunichar(c, &format!("\\mathfont{{{}}}", c)))
        })
        + [
            ('ᵢ', "i"),
            ('ⱼ', "j"),
            ('ᵣ', "r"),
            ('ₘ', "m"),
            ('ₙ', "n"),
            ('ᵒ', "o"),
            ('ₚ', "p"),
            ('ₖ', "k"),
            ('₀', "0"),
        ]
        .iter()
        .fold(rope::Rope::from(""), |ret, (sub, of)| {
            ret + rope::Rope::from(newunichar(*sub, &format!("\\textsubscript{{{}}}", of)))
        })
        + [('ᵖ', "p")]
            .iter()
            .fold(rope::Rope::from(""), |ret, (sup, of)| {
                ret + rope::Rope::from(newunichar(*sup, &format!("\\textsuperscript{{{}}}", of)))
            })
}

/*
 * set up \documentclass{...} ... \begin{document}
 */
pub fn doc_begin(title: &str, authors: Vec<&str>) -> rope::Rope {
    rope::Rope::from("")
    + rope::Rope::from("\\documentclass{article}\n".to_string())
    + packages()
    + rope::Rope::from(format!("\\author{{{}}}\n", authors.join(r"\and ")))
    + rope::Rope::from(format!("\\title{{{}}}\n", title))
    + unicode_hack()
    // r"...
    // ...
    // " the newline at the end of ... may be relevant
    + rope::Rope::from(
    r"\newfontfamily{\mathfont}{STIX2Math.otf}
    \newfontfamily{\ttmathfont}{texgyrecursor-regular.otf}
    \hypersetup{
        colorlinks,
        linkcolor={red!50!black},
        citecolor={blue!50!black},
        urlcolor={blue!80!black}
    }
    \renewcommand{\ULdepth}{2.0pt}
    \contourlength{1pt}

    \newcommand{\fancyuline}[1]{%
        \uline{\phantom{#1}}%
      \llap{\contour{white}{#1}}%
    }

    \titleformat{\paragraph}[hang]{\normalfont\normalsize\bfseries}{\fancyuline{\theparagraph}}{1em}{\fancyuline}
    \titlespacing*{\paragraph}{0pt}{2.25ex plus 1ex minus .2ex}{0pt}
    ")
   + rope::Rope::from(
   r"\begin{document}
    \maketitle
    \clearpage
    \tableofcontents
    \clearpage
    \setmainfont[
         BoldFont={STIX2Text-Bold.otf},
         ItalicFont={STIX2Text-Italic.otf},
         BoldItalicFont={STIX2Text-BoldItalic.otf}
    ]
    {STIX2Text-Regular.otf}
    ")
}

pub fn doc_end() -> rope::Rope {
    r"\end{document}".into()
}
