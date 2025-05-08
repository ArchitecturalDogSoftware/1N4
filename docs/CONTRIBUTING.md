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
within the [`LICENSE_TEMPLATE`](./LICENSE_TEMPLATE.md) file in the [`docs/`](./) directory.

## Dependencies

This project uses [`cargo-deny`](https://github.com/embarkstudios/cargo-deny) to lint its dependencies.
It checks for
duplicate/banned crates,
incompatible licenses,
untrusted sources,
security advisories,
and unmaintained/yanked dependencies,
but these checks can have false positives.
In particular, the list of allowed licenses is the minimal list of what is used by 1N4's current dependencies,
not the minimal list of what we will accept --- we are open to adding more!

## Programming Conventions

This is a non-exhaustive and relatively pedantic list of the expected rules for contributed source code.
This exists to ensure that added code is high quality, consistent, performant, and easy to understand.

If you feel that these rules are incomplete or should be modified, feel free to make a pull request,
and if there is any confusion, feel free to ask me directly or view pre-existing files within the repository.

Many of these conventions are enforced by linters and formatters,
all of which are present in GitHub Actions workflows.
Accordingly, all the lints and tests can be run without installation and configuration
using [`act`](https://nektosact.com/).

### Data

- All configuration data files should be in the TOML format (`.toml`).
- TOML files are linted and formatted by [Taplo](https://taplo.tamasfe.dev/).
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
- Rust files are linted by [rust-analyzer](https://rust-analyzer.github.io/).
  - This project is linted with [Clippy](https://doc.rust-lang.org/clippy/)
    instead of the regular `cargo check` command.
    This can be configured by setting the `rust-analyzer.check.command` option to `clippy`.
  - Lints are enabled for the entire workspace to guide you towards preferred practices.
- Rust files are formatted by [Rustfmt](https://github.com/rust-lang/rustfmt).
  - Formatting rules are already configured by the `rustfmt.toml` file at the crate root,
    and should not be modified through the command-line or otherwise.
- When ignoring lints, you *must* provide the attribute with a 'reason' field.
  - E.g., `#[expect(lint_name, reason = "why this lint is being ignored")]`.
  - Allow attributes are highly discouraged;
    expect attributes should be used instead when at all possible.
  - When ignoring the `unsafe_code` lint specifically,
    the attribute's 'reason' field must explain why the unsafe code is necessary.
- All types, fields, and methods should be fully documented,
  even if not part of the public API.
- Logic should be easy to follow.
  If something is overly confusing or obscure,
  it should be explained in a comment.
- Names must be consistent and clear.
  Single-letter names are only allowed within single-line closures.
- Code should be as safe and performant as possible.
  Avoid repeated allocations, computation, and the usage of Unsafe Rust.
  - If unsafe *is* used,
    it is **required** to have documentation explaining why it is safe.
    This should come in the form of a comment above the usage starting with "Safety:",
    followed by the reason that the code will not cause undefined behavior.
- Prefer returning results over panicking under most circumstances.
  - Panics are considered okay if they arise due to issues present at compile-time.
    For example, failing to read a file should never panic,
    but calling blocking functions within an asynchronous runtime should *always* panic.
  - Code containing `unreachable!` is expressly allowed,
    however it must be provided a string argument
    (and optionally an additional line comment)
    describing why it will not cause a panic.

#### Rust Documentation

As with source code,
refer to the Rust API Guidelines.
Specifically, refer to the [section on Documentation](https://rust-lang.github.io/api-guidelines/documentation.html).

- As with [standalone documentation](#standalone-documentation),
  prefer the use of [CommonMark](https://commonmark.org/) and
  refer to the [formatting style of mdformat](https://mdformat.readthedocs.io/en/stable/users/style.html)
  where CommonMark is not opinionated.
- Unlike standalone documentation,
  Rust documentation does *not* use Semantic Line Breaks.
  This is to avoid excessive use of vertical space in code by documentation comments.
- As with standalone documentation,
  avoid using raw HTML.
- Unlike standalone documentation,
  using the features added to CommonMark by Rust documentation is allowed.
  - Notably, linking between items is encouraged.
  - This is because Rust documentation is very standardized,
    such as Rustdoc being a standard tool for HTML documentation generation.
    This means that compatibility is not an issue.

### Standalone Documentation

This section applies to *standalone* documentation, such as this file.
See [the standards for Rust documentation](#rust-documentation) for code documentation.

- Documentation should be written in [CommonMark](https://commonmark.org/) markdown.
  - Though CommonMark allows it,
    raw HTML should be used *very* sparsely for 1N4 documentation.
- Documentation should be formatted using [mdformat](https://github.com/executablebooks/mdformat),
  which enforces an opinionated subset of CommonMark.
- All documentation should be written with [Semantic Line Breaks](https://sembr.org/).
  - We do not enforce the 80 column limit.
    If lines are getting to be longer than 80~100 columns without hyperlinks,
    it is a problem of conservative use of line breaks,
    not a problem of formatting.

Not using GitHub Flavored Markdown means that certain features,
such as checklists, tables, and strikethrough text,
are not available.
These choices were made for the sake of
wide compatibility, maintainability, and plain-text readability.
See [this issue](https://github.com/ArchitecturalDogSoftware/1N4/issues/3) for a look at the decision making process.

### Scripts

- Standalone scripts are written in Bash and located in the [`scripts/`](../scripts/) directory.
  - These are written for use on Ubuntu 24.04 and similarly equipped systems because that is what we use.
    If you would like more portable shell scripts,
    please file an issue and we can port them to the same standards as GitHub Actions workflows.
- Shell scripting inside of GitHub Actions workflows should prefer [POSIX `sh`](https://pubs.opengroup.org/onlinepubs/9799919799/utilities/V3_chap02.html),
  but this is not a strict requirement.
  At a minimum, code should work on [BusyBox](https://www.busybox.net/)
  so that workflows can run in [Alpine Linux](https://www.alpinelinux.org/) containers.
  - Notably, the `local` keyword and [process substitution](https://en.wikipedia.org/wiki/Process_substitution)
    are used despite not being in POSIX `sh`;
    because they are present in BusyBox `sh`
    and are easy to remove if true POSIX compliance is necessary.
- Shell scripts are linted by [ShellCheck](https://github.com/koalaman/shellcheck)
  and formatted by [shfmt](https://github.com/mvdan/sh).
- YAML files are formatted by [yamlfmt](https://github.com/google/yamlfmt).
- GitHub Actions workflows are linted by [actionlint](https://github.com/rhysd/actionlint),
  which also includes ShellCheck.
- Scripts should also limit themselves to lines 120 characters long.
  This is not enforced by formatters due to technical limitations,
  but should be followed by the programmer.
