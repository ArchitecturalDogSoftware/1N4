# Contribution Guidelines

Thank you for your interest in contributing to 1N4!
Below is a list of the general rules to follow when submitting code, data, assets, and other content to this bot.

We will be significantly more lenient with contributions from new users,
just be aware that We may modify any committed content to better fit the general guidelines.
And of course, as always, We will be around if you need any help.

## Commits and Pull Requests

Commit messages should be clear and concise.
They should properly describe what was added, removed, or otherwise modified within the commit.
It is also highly preferred that your commit does not contain several large changes.
Please do your best to split large commits into several smaller ones
so that changes are easier to review and revert.

Pull requests should describe in detail what is being added, removed, or otherwise changed, and why.
It is expected that they will be as sensible and informative as reasonably possible in the current context.

Neither commits nor pull requests should cause the build to fail, under any circumstances.
If a change does cause a build failure, it is expected that you will submit a fix in a follow-up commit.
Pull requests that cause a build failure will be denied until the issue is solved.

This project follows [Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html),
and any version changes must follow its rules.

## Project Licensing and Copyright Notices

Due to this project's chosen license, all submitted content falls under the [GNU Affero General Public License version 3 or later](../LICENSE).
If you wish to submit content under another license, please contact me directly beforehand.

It is also important that, when creating new file with an applicable content type,
they should have a license disclaimer prepended to the file contents.
Applicable file types include (but are not limited to)
Rust source files (`.rs`) and script files (`.sh`, `.bat`).

There is a copyright header template available to you
within the [`LICENSE_TEMPLATE`](./LICENSE_TEMPLATE.md) file in the [`docs/`](.) directory.

## Programming Conventions

This is a non-exhaustive and relatively pedantic list of the expected rules for contributed source code.
This exists to ensure that added code is high quality, consistent, performant, and easy to understand.

If you feel that these rules are incomplete or should be modified, feel free to make a pull request,
and if there is any confusion, feel free to ask me directly or view pre-existing files within the repository.

### Data

- All configuration data files should be in the TOML format (`.toml`).
- Always indent using four spaces, not tabs.
- Lines should never exceed 120 characters.
- Arrays and objects may be single-line if they do not exceed the character limit.
- Single-line objects and arrays should contain spaces as padding.
- Stored command data should be in the most sensible format.
  When in doubt, use compressed Messagepack.

### Source Code

All code should strive to follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/about.html),
but here are some key rules:

- All source files should be in the Rust file format (`.rs`).
- All types, fields, and methods should be fully documented, even if not part of the public API.
- Logic should be easy to follow.
  If something is overly confusing or obscure, it should be explained in a comment.
- Code should have consistent formatting.
  Use the included `rustfmt.toml` file as your format configuration.
- Names must be consistent and clear.
  Single-letter names are only allowed within single-line closures.
- Code should be as safe and performant as possible.
  Avoid repeated allocations, computation, and the usage of Unsafe Rust.

### Documentation

This section applies to *standalone* documentation, such as this file.
See [the standards for source code](#source-code) for code documentation.

- Documentation should be written in [CommonMark](https://commonmark.org/) markdown.
- Documentation should be formatted using [mdformat](https://github.com/executablebooks/mdformat),
  which enforces an opinionated subset of CommonMark.
- All documentation should be written with [Semantic Line Breaks](https://sembr.org/).
  - The only exception is that documentation should always wrap after 120 columns,
    instead of Semantic Line Breaks' recommendation of 80 columns.
    This is where our mdformat configuration deviates from the default,
    which does not enforce any line wrapping at all.

Not using GitHub Flavored Markdown means that certain features,
such as checklists, tables, and strikethrough text,
are not available.
These choices were made for the sake of
wide compatibility, maintainability, and plain-text readability.
See [this issue](https://github.com/Jaxydog/1N4/issues/3) for a look at the decision making process.
