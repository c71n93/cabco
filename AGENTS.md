## Agent skills

### Issue implementation branches

When asked to implement an issue, create a new branch from `master` before making implementation changes. Follow the branch naming rules in `CONTRIBUTING.md`. Use a different base or an existing branch only when the user explicitly requests it.

### Issue tracker

Issues and specs live as GitHub issues; use the `gh` CLI. Before creating an issue, read `docs/agents/issue-tracker.md` for required labels and tracker conventions.

### Triage labels

Default five-role vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

### Temporary Rust caller ordering

When adding or modifying ordinary Rust free functions, place callers before the
same-module helpers they directly call when the local callee is unambiguous.
Keep the visibility groups from `CONTRIBUTING.md`; apply caller ordering within
each group as an exception to preserving its existing order. Leave unrelated
functions unconstrained. Treat mutually recursive functions as a group with no
required internal order; self-recursion imposes no order.

Apply this manually to the functions being changed. Skip ambiguous or shadowed
names, cross-module calls, methods, trait dispatch, function pointers, macro
expansion, and generated code. Only order calls whose targets are clear without
evaluating conditional compilation.

<!-- @todo #27:60min Remove this temporary section once a dependable caller-ordering tool is found or implemented and adopted; verify visibility grouping, recursion exemptions, and skips for uncertain targets and excluded call forms.
-->
