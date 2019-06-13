extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate crowbook_text_processing;
extern crate globset;
extern crate im;
extern crate log;
extern crate logging_timer;
extern crate olean_rs as olean;
extern crate path_slash;
extern crate pathdiff;
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
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;
use std::string::String;

mod config;
mod errors;
mod path;
mod tex_gen;
use config::*;
use errors::ResultExt;
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
    /* walk over [src/a, src/z, src/], and make a set of unique files
     * It is possible that some normalization needs to occur here
     * In which place our entire sorting/globbing mechanism is pretty screwed
     * but we perhaps can say we don't support hard links and non-portable
     * filesystem features in source repositories.
     */
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
    let olean_files: Vec<OsString> = unique_files
        .iter()
        .map(|pb| pb.clone().into_os_string())
        .collect();

    let latex_tree: im::ordmap::OrdMap<&OsStr, rope::Rope> = {
        let _tmr = timer!("process lean").level(log::Level::Info);
        olean_files
            .par_iter()
            .map(|x| Ok(x.as_os_str()))
            // generate latex
            .fold(|| Ok(im::ordmap::OrdMap::new()), gen_latex)
            .reduce(
                || Ok(im::ordmap::OrdMap::new()),
                |map1, map2| Ok(im::ordmap::OrdMap::union(map1?, map2?)),
            )?
    };

    for doc in docs.documents {
        let mut glob_set_builder = globset::GlobSetBuilder::new();
        for dir in doc.src_dirs.iter() {
            let dir_name = dir.to_string_lossy();
            let glob_str = if dir_name.ends_with("/") {
                format!("{}*", dir_name)
            } else {
                format!("{}/*", dir_name)
            };
            let dir_glob = globset::Glob::new(&glob_str);
            glob_set_builder.add(dir_glob?);
        }
        let glob_set = glob_set_builder.build()?;
        let mut ropes: Vec<rope::Rope> = vec!["".into(); doc.src_dirs.len()];
        /* build sections in the order of the first src_dir that matches the glob */
        {
            let _tmr = timer!("sorting", "sections {}.tex", doc.file_name).level(log::Level::Info);
            for (file_name, latex_src) in &latex_tree {
                let matches = glob_set.matches(file_name);
                let min = matches.iter().fold(None, |x_option, y| match x_option {
                    None => Some(y),
                    Some(x) => {
                        if x < y {
                            Some(x)
                        } else {
                            Some(y)
                        }
                    }
                });
                /*                println!("file_name: {:?} first_match: {:?} matches: {:?} src_dirs: {:?}",
                                        file_name, first_match, matches, doc.src_dirs);
                */
                match min {
                    Some(index) => {
                        let foo = &ropes[*index];
                        let base = &doc.src_dirs[*index];
                        let ext_path = path::olean_to_lean(file_name);
                        let trimmed_path = pathdiff::diff_paths(&ext_path, base);
                        let section: rope::Rope = rope::Rope::from(r"\section{")
                            + escape::tex(
                                match &trimmed_path {
                                    None => ext_path.to_string_lossy(),
                                    Some(s) => s.to_string_lossy(),
                                },
                            )
                            .into()
                            + "}".into();
                        ropes[*index] = (foo.clone()) + section + latex_src.clone()
                    }
                    None => (),
                }
            }
        }
        /* collate all the sections into one document sandwiched by a header and footer */
        let tex_src_string = {
            let _tmr =
                timer!("collating", "sections {}.tex", doc.file_name).level(log::Level::Info);
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

        if doc.output_tex() {
            /* Write tex sources */
            let _tmr = timer!("writing", "{}.tex", doc.file_name).level(log::Level::Info);
            std::fs::create_dir_all(&doc.output_dir)?;
            let mut out_buf_tex = File::create(PathBuf::from_slash(format!(
                "{}/{}.tex",
                doc.output_dir, doc.file_name
            )))?;
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
                let _tmr = timer!("writing", "{}.pdf", doc.file_name).level(log::Level::Info);
                std::fs::create_dir_all(&doc.output_dir)?;
                let mut out_buf_pdf = File::create(PathBuf::from_slash(format!(
                    "{}/{}.pdf",
                    doc.output_dir, doc.file_name
                )))?;
                out_buf_pdf.write_all(&pdf_data)?
            }
        }
    }
    Ok(())
}
