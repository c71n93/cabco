# CABCO

**Code Autonomy Bounded by Component Ownership**

CABCO is a project-level policy framework for controlling AI autonomy through explicit component ownership while keeping interfaces, contracts, and sensitive modules under human control.

It aims to:
- declare components as AI- or human-owned;
- allow autonomous changes inside AI-owned components;
- require human review when changes cross declared boundaries;
- keep ownership rules explicit, version-controlled, and tool-independent.

CABCO is currently in the design stage.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the issue-linked branch and commit
naming rules applied to pull requests.

Run `cargo run -- check --base <commit> --head <commit>` in a local Git
repository (its root or a subdirectory). Both arguments are required and accept
commit IDs or references naming commits. CABCO compares those committed trees
directly, including divergent histories; working-tree edits and untracked files
are ignored. It never fetches or pushes.

This increment supports only commits without a root `cabco.yaml`. If either
selected commit contains that component policy file, evaluation fails explicitly
as unsupported until policy support lands. Without policy, existing file
versions are unassigned and every changed path requires REVIEW. Moves count as
deletion plus creation; copies count as creation of the destination.

Output groups changed paths under `REVIEW (count)` then `AUTONOMOUS (count)`,
with alphabetically sorted paths, one per line, and no explanations. Empty groups
are omitted; no changes prints `AUTONOMOUS — no changed files`. Exit codes are
0 for success without REVIEW paths, 1 for success requiring REVIEW, and 2 for
errors (printed to stderr without a verdict). AUTONOMOUS proves neither
correctness nor contract satisfaction and records no approvals.
