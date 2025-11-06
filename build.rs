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
use license_page::opt::{GetLicensesOpt, ToMarkdownPageOpt};

/// Describes the permitted values of a custom configuration.
#[expect(unused, reason = "these values may be used in the future")]
#[non_exhaustive]
enum CustomCfgValues<'s> {
    /// No value is expected.
    None,
    /// Any value is expected.
    Any,
    /// One of the specified values is expected.
    List(&'s [&'s str]),
    /// No value, or one of the specified values, is expected.
    NoneOrList(&'s [&'s str]),
}

/// Contains data for creating custom configurations.
#[non_exhaustive]
struct CustomCfg<'s> {
    /// The configuration key. This must be a valid Rust identifier.
    key: &'s str,
    /// The configuration's expected value(s).
    values: CustomCfgValues<'s>,
}

impl<'s> CustomCfg<'s> {
    /// Creates a new [`CustomCfg`].
    const fn new(key: &'s str, values: CustomCfgValues<'s>) -> Self {
        Self { key, values }
    }

    /// Informs the build system to check for this configuration during compilation.
    fn register(self) -> Self {
        fn list_string(values: &[&str]) -> String {
            values.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(", ")
        }

        println!("cargo::rustc-check-cfg=cfg(ina_{}, values({}))", self.key, match self.values {
            CustomCfgValues::None => "none()".to_string(),
            CustomCfgValues::Any => "any()".to_string(),
            CustomCfgValues::List(items) => list_string(items),
            CustomCfgValues::NoneOrList(items) => format!("none(), {}", list_string(items)),
        });

        self
    }

    /// Enables the configuration for the current compilation, assigning the provided value.
    fn set(self, value: Option<&str>) {
        match (value, &self.values) {
            (None, CustomCfgValues::None | CustomCfgValues::NoneOrList(_)) => {
                println!("cargo::rustc-cfg=ina_{}", self.key);
            }
            (Some(value), CustomCfgValues::Any) => {
                println!("cargo::rustc-cfg=ina_{}=\"{value}\"", self.key);
            }
            (Some(value), CustomCfgValues::List(items) | CustomCfgValues::NoneOrList(items))
                if items.contains(&value) =>
            {
                println!("cargo::rustc-cfg=ina_{}=\"{value}\"", self.key);
            }
            (None, CustomCfgValues::Any) => {
                println!("cargo::error=expected a value for cfg '{}'", self.key);
            }
            (None, CustomCfgValues::List(items)) => {
                println!("cargo::error=expected a value (one of {items:?}) for cfg '{}'", self.key);
            }
            (Some(value), CustomCfgValues::None) => {
                println!("cargo::error=unexpected value '{value}' for cfg '{}'", self.key);
            }
            (Some(value), CustomCfgValues::List(items)) => {
                println!("cargo::error=unexpected value '{value}' (expected one of {items:?}) for cfg '{}'", self.key);
            }
            (Some(value), CustomCfgValues::NoneOrList(items)) => {
                println!(
                    "cargo::error=unexpected value '{value}' (expected none or one of {items:?}) for cfg '{}'",
                    self.key
                );
            }
        }
    }

    /// Associates this configuration with an environment variable, assigning it to the variable's value.
    #[expect(clippy::expect_used, reason = "if the expect fails, it means there is a logic error that should be fixed")]
    #[expect(unused, reason = "this function may be used in the future")]
    fn env<P, F>(self, should_enable: P, get_value: F)
    where
        P: FnOnce(&str) -> bool,
        F: FnOnce(String) -> Option<String>,
    {
        let env_key = format!("INA_{}", self.key.to_uppercase());
        let env_value = std::env::var(&env_key);

        println!("cargo::rerun-if-env-changed={env_key}");

        if env_value.as_deref().is_ok_and(should_enable) {
            self.set(get_value(env_value.expect("this should only run if the variable is set")).as_deref());
        }
    }

    /// Associates this configuration with an environment variable, assigning it to the variable's value.
    ///
    /// If the variable's value is invalid or missing, the configuration will be set to the provided default.
    fn env_or_else<P, F, D>(self, should_use_value: P, get_value: F, default: D)
    where
        P: FnOnce(&str) -> bool,
        F: FnOnce(String) -> Option<String>,
        D: FnOnce() -> Option<&'static str>,
    {
        let env_key = format!("INA_{}", self.key.to_uppercase());
        let env_value = std::env::var(&env_key);

        println!("cargo::rerun-if-env-changed={env_key}");

        if let Ok(env_value) = env_value
            && should_use_value(&env_value)
        {
            self.set(get_value(env_value).as_deref());
        } else {
            self.set(default());
        }
    }
}

fn main() -> std::io::Result<()> {
    // Add custom `#[cfg]` entries.
    CustomCfg::new("component_validation", CustomCfgValues::List(&["relaxed", "strict"])).register().env_or_else(
        |env_value| matches!(env_value, "relaxed" | "strict"),
        Some,
        || Some("relaxed"),
    );

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

    // Assumes that the contents of `.git/HEAD` will always be either `refs/heads/BRANCH_NAME` or the commit hash.
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
    const CRATE_LICENSES_SECTION_PREAMBLE: &str = "\
These are the licenses of 1N4 and its dependencies.
We are not lawyers, but in short:
1N4 depends on code written by other people, who allow us to use and share their code under certain conditions.
These conditions are formally are formally written into software licenses.
For many of these, the restrictions boil down to just providing attribution, which this file serves to do.
For some of them, such as 1N4's license, the licenses are designed to grant you, the user, certain freedoms.
For example, you're entitled to a copy of the full source code behind any 1N4 instance (or any software based on it).

In this section, we list each license or combination of licenses (and exceptions) used by 1N4 and its dependencies.
The next section contains the full text of each license or exception.";

    println!("cargo::rerun-if-changed={}", root_dir.join("Cargo.lock"));

    let mut get_licenses_opt = GetLicensesOpt::new();
    // Don't include the dependencies only used in ["tests, examples, and benchmarks"][used_in],
    // because they're "not used when compiling a package for building" and "not propagated to
    // other packages which depend on this package," so I don't think that they're relevant to
    // built binaries.
    //
    // In an ideal world, I would enabled this if I could detect if this is currently being built
    // in "dev mode" (unrelated to the "dev" profile usually referred to as "debug"), but I think
    // that would require a hack, which I'm inclined to avoid --- I doubt the accuracy of the
    // licenses file matters for these builds.
    //
    // [used_in]: <https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#development-dependencies>
    *get_licenses_opt.avoid_dev_deps_mut() = true;
    // Assume that these _should_ be included, because though they might not be included in the
    // binary, their _output_ might be, which might include code under their license.
    *get_licenses_opt.avoid_proc_macros_mut() = false;
    *get_licenses_opt.avoid_build_deps_mut() = false;

    let mut to_markdown_page_opt = ToMarkdownPageOpt::new();
    *to_markdown_page_opt.crate_licenses_preamble_mut() = Some(CRATE_LICENSES_SECTION_PREAMBLE.to_string());

    let mut out = BufWriter::new(File::create(out_dir.join("licenses.md"))?);
    CrateList::from_crate_directory(root_dir.as_str(), get_licenses_opt)
        .to_markdown_license_page(&mut out, to_markdown_page_opt)
}
