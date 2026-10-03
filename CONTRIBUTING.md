# Contributing

Every change must be backed by a GitHub issue. Pull requests enforce the branch,
title, and issue-linkage rules below by syntax; the check does not query GitHub
to confirm that a referenced number exists, is an issue rather than a pull
request, or is open.

## Branch names

Start the branch name with one positive issue number without leading zeroes,
then a hyphen and a lowercase kebab-case description. Description segments may
contain lowercase English letters and digits.

Valid examples:

- `10-enforce-pr-naming`
- `123-fix-booking-v2`

Invalid examples include `10` (no description), `0-fix`, `01-fix`,
`10-Fix-booking`, `10-fix_booking`, `10-fix--booking`, and `10-fix-`.

## Pull request titles

Begin the PR title with one or more adjacent issue references. Use a positive
issue number without leading zeroes in each reference, include the hash sign,
place no spaces between references, and put exactly one space before a summary
whose first character is an uppercase English letter (`A`–`Z`). The first
reference must match the branch's issue number; later references may identify
additional issues addressed by the PR.

Valid examples:

- `[#123] Add booking validation`
- `[#1][#2][#3] Fix booking validation`

Invalid examples include `[123] Missing hash`, `Missing reference`,
`[#0] Zero issue`, `[#01] Leading zero`, `[#1] [#2] Spaced references`,
`[#1]  Extra space`, and `[#1] lowercase summary`.

## Pull request descriptions

Use a GitHub closing keyword for the branch's primary issue, for example
`Closes #10`, `Fixes #10`, or `Resolves #10`. This creates GitHub's issue link
and closes the issue when the PR reaches the default branch. Additional issues
may also be linked or closed in the description.

The check runs when a pull request is opened, reopened, edited, or updated. It
validates the current source branch, title, and description. It does not inspect
individual commits and does not run for direct pushes.

## Rust public API documentation

Document externally reachable Rust public items with Rustdoc comments (`///`),
and document the crate with `//!`. This includes public structs, methods,
functions, fields, enums and variants, traits, modules, and re-exported public
API covered by Rust's built-in `missing_docs` lint. Private and crate-visible
items are outside this requirement.

Describe the existing contract using the vocabulary in [CONTEXT.md](CONTEXT.md):
valid inputs, outputs, errors, and observable behavior where relevant. The lint
checks that documentation is present; reviewers still assess its quality.

The package configures `missing_docs` in `Cargo.toml` so Cargo checks fail if
public API documentation is missing.
The Public API documentation workflow runs on pull requests and pushes to
`master`, using stable Rust with Clippy. Run the same check locally from the
repository root with that toolchain selected:

```sh
cargo +stable clippy --all-targets --all-features -- -D missing_docs
```

The command checks all targets with all features enabled and fails if public
API documentation is missing.

## Review commits and merging

Use commits that help reviewers follow the work. Temporary subjects such as
`WIP` and `fixup!` are allowed because the repository uses squash-only merging;
individual PR commits do not enter `master`.

GitHub defaults the squash commit subject to the validated PR title and its body
to the PR description. The merger can still edit that generated message after
the check has run, so this policy prevents accidental formatting mistakes rather
than deliberate owner overrides. The owner retains an emergency bypass for
direct hotfixes and failed checks.
