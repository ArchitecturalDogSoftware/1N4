// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Copyright © 2025 RemasteredArch
//
// This file is part of 1N4.
//
// 1N4 is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public
// License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later
// version.
//
// 1N4 is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License along with 1N4. If not, see
// <https://www.gnu.org/licenses/>.

//! `build.rs`: 1N4's [build script].
//!
//! Currently, [`self::generate_license_page`] is all that this is used for.
//!
//! [build script]: <https://doc.rust-lang.org/cargo/reference/build-scripts.html>

use std::fs::File;
use std::io::{BufWriter, Read, Write};

use camino::{Utf8Path, Utf8PathBuf};
use license_page::CrateList;

fn main() -> std::io::Result<()> {
    // These environment variables are provided by Cargo, so they should always be present. It
    // looks like Cargo is only handling UTF-8 paths anyways, so it's safe to unwrap on that too.
    //
    // <https://github.com/rust-lang/cargo/blob/f5b3a6ba899c2eb9285dd3769aa6d84179ee7f8b/src/cargo/core/compiler/custom_build.rs#L879>
    #[expect(clippy::unwrap_used, reason = "Cargo should define this value and with only UTF-8")]
    let root_dir = Utf8PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    #[expect(clippy::unwrap_used, reason = "Cargo should define this value and with only UTF-8")]
    let out_dir = Utf8PathBuf::from(std::env::var("OUT_DIR").unwrap());

    self::generate_build_information(&root_dir, &out_dir)?;
    self::generate_license_page(&root_dir, &out_dir)
}

/// Generates a file (`$OUT_DIR/build_info.rs`) containing various pieces of information about this
/// build.
///
/// This is positioned _before_ [`self::generate_license_page`] so that this runs every time 1N4 is
/// built.
///
/// Specifically, it provides the following constants, in a format like this example output:
///
/// ```ignore
/// /// The current Git commit hash 1N4 was built at.
/// pub const COMMIT_HASH: &str = "5f78bcf0be41b4042cf5d20f26ae4aa80d84ee72";
/// /// The current Cargo features 1N4 was built with.
/// pub const FEATURES: &str = "default,dotenv";
/// /// The current target triple 1N4 was built for.
/// pub const TARGET_TRIPLE: &str = "x86_64-unknown-linux-gnu";
/// /// The current profile (debug or release) 1N4 was built with.
/// pub const PROFILE: &str = "debug";
/// ```
fn generate_build_information(root_dir: &Utf8Path, out_dir: &Utf8Path) -> std::io::Result<()> {
    let mut out = BufWriter::new(File::create(out_dir.join("build_info.rs"))?);

    let current_commit = get_current_commit(root_dir)?;
    writeln!(
        out,
        r#"/// The current Git commit hash 1N4 was built at.
        pub const COMMIT_HASH: &str = "{current_commit}";"#
    )?;

    #[expect(clippy::unwrap_used, reason = "Cargo should define this value and with only UTF-8")]
    let features = std::env::var("CARGO_CFG_FEATURE").unwrap();
    writeln!(
        out,
        r#"/// The current Cargo features 1N4 was built with.
        pub const FEATURES: &str = "{features}";"#
    )?;

    #[expect(clippy::unwrap_used, reason = "Cargo should define this value and with only UTF-8")]
    let target_triple = std::env::var("TARGET").unwrap();
    writeln!(
        out,
        r#"/// The current target triple 1N4 was built for.
        pub const TARGET_TRIPLE: &str = "{target_triple}";"#
    )?;

    #[expect(clippy::unwrap_used, reason = "Cargo should define this value and with only UTF-8")]
    let profile = std::env::var("PROFILE").unwrap();
    writeln!(
        out,
        r#"/// The current profile (debug or release) 1N4 was built with.
        pub const PROFILE: &str = "{profile}";"#
    )?;

    Ok(())
}

/// Fetches the current commit directly hash from the `.git` directory at `root_dir`.
fn get_current_commit(root_dir: &Utf8Path) -> std::io::Result<String> {
    let git_dir = root_dir.join(".git");

    let mut head_ref = String::new();
    File::open(git_dir.join("HEAD"))?.read_to_string(&mut head_ref)?;
    // Trim the trailing line ending in the file.
    let head_ref = head_ref.trim_ascii_end();

    // Assumes that the contents of `.git/HEAD` will always be either
    // `refs/heads/BRANCH_NAME` or the commit hash.
    let Some(current_branch_path) = head_ref.strip_prefix("ref: ") else {
        return Ok(head_ref.to_string());
    };

    let mut current_commit = String::new();
    File::open(git_dir.join(current_branch_path))?.read_to_string(&mut current_commit)?;

    // Trim the trailing line ending in the file.
    Ok(current_commit.trim_ascii_end().to_string())
}

/// Generates a Markdown file that contains the declared licenses and their full texts of 1N4 and
/// its dependencies.
///
/// This includes both direct and transitive dependencies, but only includes `dev-dependencies` in
/// builds with `debug_assertions` enabled and never includes `build-dependencies`. The Markdown
/// file is generated in [CommonMark](https://commonmark.org/) Markdown and is located at
/// `$OUT_DIR/licenses.md`.
fn generate_license_page(root_dir: &Utf8Path, out_dir: &Utf8Path) -> std::io::Result<()> {
    println!("cargo::rerun-if-changed={}", root_dir.join("Cargo.lock"));

    let mut out = BufWriter::new(File::create(out_dir.join("licenses.md"))?);
    CrateList::from_crate_directory(root_dir.as_str()).to_markdown_license_page(&mut out)
}
