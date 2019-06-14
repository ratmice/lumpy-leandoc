# lumpy-leandoc
lean doc_string latex generator

This is a somewhat experimental documentation generator for
lean source files. In order to avoid parsing lean sources it relies
on extracting the documentation from lean bytecode.

Due to the above, you must your **compile sources** with **_lean first_**.
Documentation will then be generated from the .olean file which lean produces.

See Lumpy.toml for an example configuration file.

# Installation
  1. install [rust](https://www.rust-lang.org/tools/install)
  2. ```cargo install --git https://github.com/ratmice/lumpy-leandoc```

# Usage
  * Add a Lumpy.toml next to your leanpkg.toml
  * `leanpkg build` as normal
  * run `lumpy-leandoc` 
	
# Output

    [examples.pdf](https://gist.github.com/ratmice/29b869369ec02232b80dce3498a4c0b4)

List of features:
  * Syntax highlighting via syntect
  * Tex -> PDF generation via tectonic
  * Markdown via pulldown_cmark

