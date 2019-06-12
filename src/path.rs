use std::fs::File;
use std::path::{Component, Path, PathBuf};
extern crate walkdir;

pub fn findup<P: AsRef<Path>>(foo: P, file: P) -> Result<(PathBuf, File), failure::Error> {
    let mut try_path: PathBuf = foo.as_ref().to_path_buf();
    try_path.push(file.as_ref());
    let file_result = File::open(try_path.clone());

    match file_result {
        Ok(file) => Ok((try_path, file)),
        Err(_) => {
            let parent = foo.as_ref().parent();
            match parent {
                Some(parent) => findup(parent, file.as_ref()),
                None => Err(format_err!("Cannot find file: {}", file.as_ref().display())),
            }
        }
    }
}

pub fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name();

    name != "." && name != "./" && name.to_str().map(|s| s.starts_with(".")).unwrap_or(false)
}

pub fn is_dir(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
}

pub fn is_olean(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_file()
        && entry
            .file_name()
            .to_str()
            .map(|s| s.ends_with(".olean"))
            .unwrap_or(false)
}

pub fn olean_to_lean<P: AsRef<Path>>(foo: P) -> PathBuf {
    let foo = foo.as_ref();
    let ext: Option<&str> = foo.extension().map_or(None, |ext| ext.to_str());
    if Some("olean") == ext {
        foo.with_extension("lean")
    } else {
        foo.to_path_buf()
    }
}

/*
 * for walking multiple directories that may overlap...
 */
pub fn walk_without_duplicates<P: AsRef<Path>>(
    set: Result<im::HashSet<PathBuf>, failure::Error>,
    p: P,
) -> Result<im::HashSet<PathBuf>, failure::Error> {
    let w = walkdir::WalkDir::new(p);
    w.into_iter()
        .filter_entry(|entry| !is_hidden(entry) && (is_dir(entry) || is_olean(entry)))
        .fold(set, |set, entry| {
            let entry = entry?;
            if is_dir(&entry) {
                set
            } else {
                Ok(set?.update(entry.into_path()))
            }
        })
}

/* This doesn't work, components normalizes out curdir except when it's the first
 * path component :(
 */
#[allow(dead_code)]
pub fn trim_at_curdir_left<P: AsRef<Path>>(foo: P) -> PathBuf {
    let mut leading = PathBuf::new();
    let _debug = foo.as_ref().components().collect::<Vec<_>>();

    for component in foo.as_ref().components() {
        match component {
            Component::CurDir => break,
            Component::RootDir => leading.push(Component::RootDir),
            Component::Normal(it) => leading.push(Component::Normal(it)),
            Component::ParentDir => leading.push(Component::ParentDir),
            Component::Prefix(it) => leading.push(Component::Prefix(it)),
        }
    }
    leading
}
