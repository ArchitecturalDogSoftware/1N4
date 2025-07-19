//! `build.rs`: 1N4's [build script].
//!
//! Currently, [`self::generate_license_page`] is all that this is used for.
//!
//! [build script]: <https://doc.rust-lang.org/cargo/reference/build-scripts.html>

use std::fs::File;
use std::io::BufWriter;

use camino::Utf8PathBuf;
use license_page::CrateList;

fn main() -> std::io::Result<()> {
    generate_license_page()
}

/// Generates a Markdown file that contains the declared licenses and their full texts of 1N4 and
/// its dependencies.
///
/// This includes both direct and transitive dependencies, but only includes `dev-dependencies` in
/// builds with `debug_assertions` enabled and never includes `build-dependencies`. The Markdown
/// file is generated in [CommonMark](https://commonmark.org/) Markdown and is located at
/// `$OUT_DIR/licenses.md`.
fn generate_license_page() -> std::io::Result<()> {
    // These environment variables are provided by Cargo, so they should always be present. It
    // looks like Cargo is only handling UTF-8 paths anyways, so it's safe to unwrap on that too.
    //
    // <https://github.com/rust-lang/cargo/blob/f5b3a6ba899c2eb9285dd3769aa6d84179ee7f8b/src/cargo/core/compiler/custom_build.rs#L879>
    #[expect(clippy::unwrap_used, reason = "Cargo should define this value and with only UTF-8")]
    let root_dir = Utf8PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    #[expect(clippy::unwrap_used, reason = "Cargo should define this value and with only UTF-8")]
    let out_dir = Utf8PathBuf::from(std::env::var("OUT_DIR").unwrap());

    println!("cargo::rerun-if-changed={}", root_dir.join("Cargo.lock"));

    let mut out = BufWriter::new(File::create(out_dir.join("licenses.md"))?);
    CrateList::from_crate_directory(root_dir.as_str()).to_markdown_license_page(&mut out)
}
