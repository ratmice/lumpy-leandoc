# lumpy-leandoc
lean doc_string latex generator

This is a somewhat experimental documentation generator for
lean source files. In order to avoid parsing lean sources it relies
on extracting the documentation from lean bytecode.

Due to the above, you must your **compile sources** with **_lean first_**.
Documentation will then be generated from the .olean file which lean produces.

See Lumpy.toml for an example configuration file.

List of features:
  * Syntax highlighting via syntect
  * Tex -> PDF generation via tectonic
  * Markdown via pulldown_cmark

