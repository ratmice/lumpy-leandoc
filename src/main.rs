extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate crowbook_text_processing;
extern crate im;
extern crate log;
extern crate logging_timer;
extern crate olean_rs as olean;
extern crate path_slash;
extern crate rayon;
// need to feature gate this linking in rayon_logs or rayon.
//extern crate rayon_logs as rayon;
extern crate tectonic;
extern crate toml;
extern crate xi_rope as rope;
use crowbook_text_processing::escape;
use path_slash::PathBufExt;

use logging_timer::timer;

use rayon::iter::ParallelIterator;
use rayon::prelude::*;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::string::String;
mod config;
mod errors;
mod html_gen;
mod path;
mod syntax_hilight;
mod tex_gen;

use config::*;
use errors::ResultExt;
use html_gen::*;
use path::*;
use tex_gen::*;

fn main() -> Result<(), failure::Error> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let cwd = std::env::current_dir()?;
    let (cfg_file_path, fd) = findup(cwd.clone(), PathBuf::from("Lumpy.toml"))?;

    let mut buf_reader = BufReader::new(fd);
    let mut config_string = String::new();
    buf_reader.read_to_string(&mut config_string)?;

    let value: Result<config::Document, _> = toml::from_str(&config_string);
    let docs: Docs = match value {
        Ok(value) => Docs {
            documents: vec![value],
        },
        Err(_) => toml::from_str(&config_string)?,
    };

    let cfg_file_dir = cfg_file_path.parent().unwrap_or_else(|| &cwd);
    env::set_current_dir(cfg_file_dir)?;

    let olean_files: Vec<PathBuf> = {
        let unique_files: im::HashSet<PathBuf> = docs
            .documents
            .par_iter()
            .fold(
                || Ok(im::HashSet::new()),
                |uniq, doc| {
                    doc.src_dirs.iter().fold(uniq, |uniq, path| {
                        walk_without_duplicates(uniq, PathBuf::from_slash_lossy(path))
                    })
                },
            )
            .reduce(|| Ok(im::HashSet::new()), |a, b| Ok(a?.union(b?)))?;

        /* It would perhaps be nice to avoid this pass */
        unique_files.iter().map(|pb| pb.clone()).collect()
    };

    for doc in &docs.documents {
        if doc.output_tex() {
            let latex_tree: im::ordmap::OrdMap<&Path, rope::Rope> = {
                let _tmr = timer!("olean -> latex").level(log::Level::Info);
                olean_files
                    .par_iter()
                    .map(|x| Ok(x.as_path()))
                    // generate latex
                    .fold(|| Ok(im::ordmap::OrdMap::new()), gen_latex)
                    .reduce(
                        || Ok(im::ordmap::OrdMap::new()),
                        |map1, map2| Ok(im::ordmap::OrdMap::union(map1?, map2?)),
                    )?
            };

            let mut ropes: Vec<rope::Rope> = vec!["".into(); doc.src_dirs.len()];
            /* build sections in the order of the first src_dir that matches the glob */
            {
                let _tmr =
                    timer!("sorted", "sections {}.tex", doc.file_name).level(log::Level::Info);
                for (file_name, latex_src) in &latex_tree {
                    for (i, src_dir) in doc.src_dirs.iter().enumerate() {
                        if file_name.starts_with(src_dir) {
                            let path = path::olean_to_lean(file_name.strip_prefix(src_dir)?);
                            let section: rope::Rope = rope::Rope::from(r"\section{")
                                + escape::tex(path.to_string_lossy()).into()
                                + "}".into();
                            let foo = &ropes[i];
                            ropes[i] = (foo.clone()) + section + latex_src.clone();
                            break;
                        }
                    }
                }
            }
            /* collate all the sections into one document sandwiched by a header and footer */
            let tex_src_string = {
                let _tmr =
                    timer!("collated", "sections {}.tex", doc.file_name).level(log::Level::Info);
                String::from(
                    ropes.iter().fold(
                        tex_gen::doc_begin(
                            &doc.title,
                            doc.authors.iter().map(String::as_str).collect(),
                        ),
                        |folding, section| folding + section.clone(),
                    ) + tex_gen::doc_end(),
                )
            };

            {
                /* Write tex sources */
                let _tmr = timer!("wrote", "{}.tex", doc.file_name).level(log::Level::Info);
                std::fs::create_dir_all(&doc.output_dir)?;
                let out_file_name = PathBuf::from_slash(format!(
                    "{}/{}.tex",
                    // FiXME unwrap
                    doc.output_dir.to_str().unwrap(),
                    doc.file_name
                ));
                let mut out_buf_tex = File::create(out_file_name)?;
                out_buf_tex.write_all(tex_src_string.as_bytes())?
            }

            if doc.output_pdf() {
                /* Run the TeX engine */
                let pdf_data: Vec<u8> = {
                    let _tmr = timer!("generate", "{}.pdf", doc.file_name).level(log::Level::Info);
                    tectonic::latex_to_pdf(tex_src_string).sync()?
                };

                /* output the results */
                {
                    let _tmr = timer!("wrote", "{}.pdf", doc.file_name).level(log::Level::Info);
                    std::fs::create_dir_all(&doc.output_dir)?;
                    let mut out_buf_pdf = File::create(PathBuf::from_slash(format!(
                        "{}/{}.pdf",
                        // FIXME unwrap
                        doc.output_dir.to_str().unwrap(),
                        doc.file_name
                    )))?;
                    out_buf_pdf.write_all(&pdf_data)?
                }
            }
        }

        if doc.output_html() {
            let html_tree: im::ordmap::OrdMap<&Path, rope::Rope> = {
                let _tmr = timer!("olean -> html").level(log::Level::Info);
                olean_files
                    .par_iter()
                    .map(|x| Ok(x.as_path()))
                    .fold(|| Ok(im::ordmap::OrdMap::new()), gen_html)
                    .reduce(
                        || Ok(im::ordmap::OrdMap::new()),
                        |map1, map2| Ok(im::ordmap::OrdMap::union(map1?, map2?)),
                    )?
            };

            let _tmr = timer!("wrote", "{}/ html", doc.file_name).level(log::Level::Info);
            for (file_name, html_src) in &html_tree {
                for (_i, src_dir) in doc.src_dirs.iter().enumerate() {
                    if file_name.starts_with(src_dir) {
                        let _path = path::olean_to_lean(file_name.strip_prefix(src_dir)?);
                        let mut output_path = doc.output_dir.clone();
                        output_path.push(&doc.file_name); // file_name here is a directory name.
                        _path.parent().map(|p| output_path.push(p));
                        std::fs::create_dir_all(&output_path)?;
                        let out_file_name = PathBuf::from_slash(format!(
                            "{}/{}.html",
                            // FIXME unwrap's
                            output_path.to_str().unwrap(),
                            _path.file_stem().unwrap().to_str().unwrap()
                        ));
                        let mut out_buf_html = File::create(out_file_name)?;
                        out_buf_html.write_all(html_src.to_string().as_bytes())?;
                        break;
                    }
                }
            }

            use petgraph::graphmap::DiGraphMap;

            let _tmr = timer!("wrote", "index.html").level(log::Level::Info);
            let mut g = DiGraphMap::<&Path, ()>::new();
            let empty_path = Path::new("");
            let empty_node = g.add_node(empty_path);

            /* The goal here is to
             * Avoid the following:
             * A/
             * A/bar.lean
             * A/baz/
             * A/foo.lean
             *
             * Rather, than what i'm prefer:
             *
             * A/
             * A/baz/
             * A/bar.lean
             * A/foo.lean
             *
             */
            /* To achieve that we:
             *   First add all the directories to the graph. */
            for file_name in html_tree.keys() {
                for src_dir in doc.src_dirs.iter() {
                    if file_name.starts_with(src_dir) {
                        let mut child_path: Option<&Path> = None;
                        let child = file_name.strip_prefix(src_dir)?.parent().unwrap();
                        for parent_path in child.ancestors() {
                            // by making a relative path (via strip_prefix),
                            // The last iteration of this loop should behave as the following.
                            //  path_node = g.add_node(empty_path);
                            // Giving us empty_path as the root of the tree..
                            let parent_node = g.add_node(parent_path);
                            if let Some(child_node) = child_path {
                                // A -> A/Baz
                                g.add_edge(parent_node, child_node, ());
                            }
                            child_path = Some(parent_node);
                        }
                    }
                }
            }

            /*
             * Then add all the Dir -> file edges
             */
            for file_name in html_tree.keys() {
                for src_dir in doc.src_dirs.iter() {
                    if file_name.starts_with(src_dir) {
                        let file_name = file_name.strip_prefix(src_dir)?;
                        let file_node = g.add_node(&file_name);
                        let parent_name = file_name.parent().unwrap();
                        let parent_node = g.add_node(&parent_name);
                        // A -> A/bar.lean
                        g.add_edge(&parent_node, &file_node, ());
                    }
                }
            }

            /*
             * With that in mind the DFS, should traverse edges through all the directories
             * before encountering edges to files.
             */
            use petgraph::visit::depth_first_search;
            use petgraph::visit::DfsEvent;

            let mut output_path = doc.output_dir.clone();
            output_path.push(&doc.file_name); // file_name here is a directory name
            let out_file_name = PathBuf::from_slash(format!(
                "{}/index.html",
                // FIXME unwrap's
                output_path.to_str().unwrap()
            ));
            std::fs::create_dir_all(&output_path)?;
            let mut out_buf_html = File::create(out_file_name)?;
            write!(
                out_buf_html,
                r#"<html><head><link rel="stylesheet" href="docs_style.css"></head><body>"#
            )?;
            write!(out_buf_html, "<ul id=\"index_root_ul\">\n")?;
            depth_first_search(&g, Some(empty_node), |event| match event {
                DfsEvent::Discover(n, _time) => {
                    if n == empty_path {
                        return Ok(());
                    }
                    if None == n.extension() {
                        write!(
                            out_buf_html,
                            "<li>\n<span class=\"caret\">{}</span>\n<ul class=\"nested\">\n",
                            n.file_name().unwrap().to_str().unwrap()
                        )
                    } else {
                        let file_stem = n.file_stem().unwrap().to_str().unwrap();
                        write!(
                            out_buf_html,
                            "<li><a href=\"{}/{}.html\">{}</a><li>\n",
                            n.parent().unwrap().display(),
                            file_stem,
                            file_stem
                        )
                    }
                }
                DfsEvent::Finish(n, _time) => {
                    if n != empty_path && None == n.extension() {
                        return write!(out_buf_html, "</ul>\n");
                    }
                    Ok(())
                }
                _ => Ok(()),
            })?;
            write!(out_buf_html, "</ul>\n")?;
            write!(
                out_buf_html,
                r#"<script>var toggler = document.getElementsByClassName("caret")
var i;

for (i = 0; i < toggler.length; i++) {{
  toggler[i].addEventListener("click", function() {{
    this.parentElement.querySelector(".nested").classList.toggle("active");
    this.classList.toggle("caret-down");
  }});
}}</script>"#
            )?;
            write!(out_buf_html, "</body></html>")?;
            /*
            // Debuging junk.
            {
                use petgraph::dot::Dot;
                let dot_path = PathBuf::from("index.dot");
                let mut dot_file = File::create(dot_path)?;
                write!(dot_file, "{:?}", Dot::with_config(&g, &[]))?;
            }
            */
        }
    }
    Ok(())
}
