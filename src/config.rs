use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Debug, Serialize)]
pub struct Document {
    pub file_name: String,
    pub title: String,
    pub authors: Vec<String>,
    pub src_dirs: Vec<PathBuf>,
    pub strip_prefix: Option<String>,
    pub output_dir: PathBuf,
    output_formats: Vec<OutputTarget>,
    comment_format: String,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Docs {
    pub documents: Vec<Document>,
}

#[derive(Deserialize, Debug, Serialize, PartialEq)]
enum OutputTarget {
    #[serde(rename = "pdf")]
    Pdf,
    #[serde(rename = "tex")]
    TeX,
    #[serde(rename = "html")]
    HTML,
}

pub trait OutputFormatStuff {
    fn output_pdf(&self) -> bool;
    fn output_html(&self) -> bool;
    fn output_tex(&self) -> bool;
}

impl OutputFormatStuff for Document {
    fn output_pdf(&self) -> bool {
        for target in self.output_formats.iter() {
            match target {
                OutputTarget::Pdf => return true,
                _ => continue,
            }
        }

        return false;
    }
    fn output_tex(&self) -> bool {
        for target in self.output_formats.iter() {
            match target {
                OutputTarget::TeX => return true,
                _ => continue,
            }
        }
        return false;
    }
    fn output_html(&self) -> bool {
        for target in self.output_formats.iter() {
            match target {
                OutputTarget::HTML => return true,
                _ => continue,
            }
        }
        return false;
    }
}
