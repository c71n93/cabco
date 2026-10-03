# PR size workflow maintenance

The workflow pins CodelyTV/pr-size-labeler v1.12.0 at commit
`19c335e7695ba922de938806dd129f0a9b992644`.

At that revision, `src/labeler.sh` compares each bucket with strict `-lt`.
Consequently, the configured largest non-failing threshold of 501 allows 500
counted additions plus deletions and assigns 501 or more to `size/xl`, which
fails the job.

When an exclusion is configured, `src/github.sh` requests changed files in
100-file pages until it receives a short page. It may stop earlier only after
the count reaches the largest configured threshold. For this workflow that
means a complete count for every passing PR and a proven lower bound of at
least 501 for an early failure; a PR cannot pass because later pages were not
read. The literal `Cargo.lock` pattern matches only the repository-root file.

## Required-check rollout

GitHub only runs a `pull_request_target` workflow when its workflow file exists
on the default branch. Requiring this new check before that first installation
would leave it permanently expected and block unrelated pull requests.

After `.github/workflows/pr-size.yml` is installed on `master`, edit the active
repository ruleset **Require PR metadata on master** and add
`Enforce 500-line PR size limit` to its required status checks. Preserve the
existing `Validate PR metadata` requirement and every other ruleset setting.
Until that update is made, labeling and failure behavior are configured but the
size check is not required for merging.
