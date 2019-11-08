# lumpy-leandoc
lean doc_string latex generator

This is a somewhat experimental documentation generator for
lean source files. In order to avoid parsing lean sources it relies
on extracting the documentation from lean bytecode.

Due to the above, you must your **compile sources** with **_lean first_**.
Documentation will then be generated from the .olean file which lean produces.

See Lumpy.toml for an example configuration file.

## Motivation
The primary motivation is to make it easy to install and use.
as such it comes with a TeX renderer [tectonic](https://tectonic-typesetting.github.io/)
which automatically downloads any latex packages and fonts needed
to render the document.

Syntax highlighting will not shell out to any external tools that need to be
installed seperately.

## Issues:
In it's current state, this automatic downloading results in the first run
taking quite a bit of time while it fills in the cache. During this time there is currently no output.
This wait can be avoided by not using the pdf output format.

## Installation
  1. install [rust](https://www.rust-lang.org/tools/install)
  2. install [harfbuzz](https://harfbuzz.org) 1.4 or later is required.
     harfbuzz needs to be built with libicu and graphite2 support.
     With ubuntu ```apt-get install libharfbuzz-dev```
     On fedora ```dnf install harfbuzz-devel``` should suffice.
  3. ```cargo install --git https://github.com/ratmice/lumpy-leandoc```

## Usage
  * Add a Lumpy.toml next to your leanpkg.toml
  * `leanpkg build` as normal
  * run `lumpy-leandoc` 
  * example output [examples.pdf](https://gist.github.com/ratmice/29b869369ec02232b80dce3498a4c0b4)

List of features:
  * Syntax highlighting via syntect
  * Tex -> PDF generation via tectonic
  * Markdown via pulldown_cmark
  * HTML output (work in progress)

