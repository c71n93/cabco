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

## Pull request size

Pull requests may contain at most 500 changed lines, counted as additions plus
deletions, with some exclusions defined in the workflow configuration. This
maintenance ceiling applies regardless of maintenance ownership.

Agents must plan smaller, coherent implementation increments, each accompanied
by its relevant tests, rather than presenting an oversized change for review.

## Deferred work with 0pdd

[0pdd](https://github.com/yegor256/0pdd) turns source-code puzzles into GitHub
issues. When deferring work, leave a comment that references the originating
issue, estimates the effort, and describes the remaining work. For example:

```rust
// @todo #30:30min Describe the concrete remaining work and its acceptance criteria.
```

Replace the issue number and description with the real work being deferred.
For multiline descriptions, indent continuation text one extra space after the
comment prefix. See the [pdd syntax guide](https://github.com/cqfn/pdd#how-to-format).

The `Check PDD puzzles` CI job validates puzzle formatting on pull requests and
pushes to `master`. To run the same parser locally, install Ruby and libmagic,
then run from the repository root:

```sh
gem install pdd
pdd -f /dev/null
```

The root `.pdd` file excludes build output, local environment files, Git metadata,
and this guide's example. `.0pdd.yml` labels generated issues `needs-triage`;
a maintainer triages them before implementation.

Repository administrators must keep an active GitHub webhook with payload URL
`https://www.0pdd.com/hook/github`, content type `application/json`, SSL verification
enabled, and only the `push` event selected. This webhook is already configured
for this public repository; no bot invitation or Actions secret is required.
GitHub webhook settings are separate from the files in this PR.

After a puzzle reaches `master`, the webhook triggers 0pdd to open its issue.
Remove the puzzle when the work is complete; 0pdd closes its issue after the
removal reaches `master`. Avoid closing generated issues manually while their
puzzles remain in the code. Check the README badge for puzzle status and the
webhook's recent deliveries or 0pdd's commit comments when troubleshooting.

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

Clippy also requires structured Rustdoc sections for the applicable public API:

- `# Errors` describes failure conditions for functions returning `Result`
  (`clippy::missing_errors_doc`).
- `# Panics` describes panic conditions detected by Clippy
  (`clippy::missing_panics_doc`).
- `# Safety` describes the caller's obligations for unsafe functions and traits
  (`clippy::missing_safety_doc`).

These lints are individually denied in `Cargo.toml` and enforced by the existing
Clippy workflows. They check section presence; reviewers must still assess
whether the documented contract is accurate and complete. Describe argument and
return-value semantics where useful; these lints do not require `# Arguments`
or `# Returns` sections.

## Rust formatting and Clippy

The Rust checks workflow runs on pull requests and pushes to `master`. Its
`Check Rust formatting` job checks formatting across the workspace without
rewriting files. Its `Check Clippy warnings` job checks the workspace, all targets,
and all features, treating warnings as errors and explicitly retaining the
missing-public-documentation requirement. The existing Public API documentation,
PR naming, and PR size workflows remain in place.

Toolchain policy: use the latest stable Rust release with rustfmt and Clippy,
rather than a pinned compiler version. New stable releases can introduce
formatting changes or new warnings. Update the local stable toolchain before
reproducing CI failures, then run the same commands from the repository root:

```sh
rustup update stable
rustup component add --toolchain stable rustfmt clippy
cargo +stable fmt --all -- --check
cargo +stable clippy --workspace --all-targets --all-features -- -D warnings -D missing_docs
```

To apply formatting locally, run `cargo +stable fmt --all`, then rerun the checks.

Adding these workflows makes failures visible; it does not configure merge
protection. For merge-blocking enforcement, select `Check Rust formatting` and
`Check Clippy warnings` as required status checks in branch protection or a
ruleset, alongside any existing required checks (including
`Check public API documentation`). This increment does not change repository
protection settings.

## Review commits and merging

Use commits that help reviewers follow the work. Temporary subjects such as
`WIP` and `fixup!` are allowed because the repository uses squash-only merging;
individual PR commits do not enter `master`.

GitHub defaults the squash commit subject to the validated PR title and its body
to the PR description. The merger can still edit that generated message after
the check has run, so this policy prevents accidental formatting mistakes rather
than deliberate owner overrides. The owner retains an emergency bypass for
direct hotfixes and failed checks.

## Rust function visibility order

From the repository root, run the same pinned command as CI (requires Node/npm):

```sh
npx --yes --package @ast-grep/cli@0.50.0 ast-grep scan --rule .github/function-order.yml
```

Within each module and inherent `impl`, place bare `pub` functions first,
restricted functions next (`pub(crate)`, `pub(super)`, `pub(in ancestor)`), and
private functions last. `pub(self)` and `pub(in self)` are private. Preserve order
within each group and move documentation and attributes with each function.

Scopes are checked independently, even with unrelated declarations between
functions. Traits, trait implementations, functions inside function bodies and
macro expansions are excluded. Generated outputs under `target/` or `generated/`,
`*.generated.rs`, and Git-ignored files are skipped. Source behind `cfg` attributes
is checked regardless of the selected build configuration.

Violations report the rule, file and line and exit with status 1. The check runs
on pull requests and pushes to `master`; it does not rewrite source.
For exceptional layouts, put a reason above a rule-specific suppression directly
before the lower-visibility function (after its documentation and attributes):

```rust
// Reason: preserve a published example's layout.
// ast-grep-ignore: rust-function-visibility-order
fn helper() {}
pub fn api() {}
```
