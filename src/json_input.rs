use serde_derive::Deserialize;

/* This is probably missing fields,
 * and probably has fields which should be options,
 * it needs a good looking at, and should be run over a larger set of inputes.
 */

#[derive(Deserialize)]
pub enum JsonLeanKind {
    #[serde(rename = "lemma")]
    Lemma,
    #[serde(rename = "theorem")]
    Theorem,
    #[serde(rename = "definition")]
    Definition,
    // FIXME ?
    #[serde(rename = "example")]
    Example,
    #[serde(rename = "structure")]
    Structure,
    #[serde(rename = "inductive")]
    Inductive,
    #[serde(rename = "instance")]
    Instance,
    #[serde(rename = "eliminator")]
    Eliminator,
    #[serde(rename = "class")]
    Class,
    #[serde(rename = "constructor")]
    Constructor,
}

#[derive(Deserialize)]
pub struct JsonLeanSource {
    pub column: u32,
    pub file: String,
    pub line: u32,
}

#[derive(Deserialize)]
pub struct JsonLeanDecl {
    pub kind: JsonLeanKind,
    pub source: JsonLeanSource,
    pub doc: Option<String>,
    pub text: String,
    #[serde(rename = "type")]
    pub typ: String,
}

#[derive(Deserialize)]
pub struct JsonLeanModule {
    pub declarations: Vec<JsonLeanDecl>,
    pub doc: Option<String>,
    pub module: String,
}
