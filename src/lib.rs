//! Evaluate immutable committed snapshots independently of Git collection and CLI output.
use std::collections::{BTreeMap, BTreeSet};
pub mod git;

/// Complete tracked files and root policy at one full selected commit ID.
#[derive(Debug)]
pub struct Snapshot {
    commit: String,
    files: BTreeMap<String, String>,
    policy: Option<Vec<u8>>,
}
impl Snapshot {
    /// Build a snapshot from repository-relative paths and opaque version identities.
    /// Identities must distinguish content and file type; they need not come from Git.
    /// Accepts 40- or 64-digit hexadecimal commit IDs, preserving their spelling.
    ///
    /// # Errors
    /// Rejects non-full commit IDs, invalid or conflicting paths, empty identities, or inconsistent policy presence.
    pub fn new(
        commit: String,
        files: BTreeMap<String, String>,
        policy: Option<Vec<u8>>,
    ) -> Result<Self, String> {
        if ![40, 64].contains(&commit.len()) || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!("invalid full commit ID {commit:?}"));
        }
        if files.iter().any(|(path, identity)| {
            identity.is_empty()
                || path.contains('\0')
                || path.split('/').any(|part| matches!(part, "" | "." | ".."))
                || path
                    .match_indices('/')
                    .any(|(end, _)| files.contains_key(&path[..end]))
        }) || files.contains_key("cabco.yaml") != policy.is_some()
        {
            return Err(format!("invalid snapshot files or policy at {commit}"));
        }
        Ok(Self {
            commit,
            files,
            policy,
        })
    }
    /// Every tracked repository-relative path, including unchanged files, alphabetically.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }
    /// Exact root policy bytes, absent when the snapshot has no component policy.
    pub fn policy(&self) -> Option<&[u8]> {
        self.policy.as_deref()
    }
    fn require_supported(&self) -> Result<(), String> {
        if self.policy.is_some() {
            return Err(format!(
                "component policy cabco.yaml at {} is unsupported",
                self.commit
            ));
        }
        Ok(())
    }
    fn changes_to<'a>(&'a self, head: &'a Self) -> impl Iterator<Item = PathClassification<'a>> {
        let base = self;
        base.files
            .keys()
            .chain(head.files.keys())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(move |path| base.files.get(*path) != head.files.get(*path))
            .map(move |path| PathClassification {
                path,
                before: base.version(path),
                after: head.version(path),
            })
    }
    fn version(&self, path: &str) -> Option<PathDetails<'_>> {
        self.files.contains_key(path).then_some(PathDetails {
            commit: &self.commit,
        })
    }
}
/// Two complete snapshots, compared directly without rename or copy inference.
#[derive(Debug)]
pub struct Comparison {
    base: Snapshot,
    head: Snapshot,
}
impl Comparison {
    /// Select the base and head snapshots; construction performs no I/O.
    pub fn new(base: Snapshot, head: Snapshot) -> Self {
        Self { base, head }
    }
    /// Inspect complete base and head snapshots, respectively.
    pub fn snapshots(&self) -> (&Snapshot, &Snapshot) {
        (&self.base, &self.head)
    }
    /// Evaluate both policies before classifying any changes, including empty comparisons.
    ///
    /// # Errors
    /// Identifies the selected commit if either snapshot contains unsupported policy.
    pub fn evaluate(self) -> Result<Classification, String> {
        self.base.require_supported()?;
        self.head.require_supported()?;
        Ok(Classification(self))
    }
}
/// Reusable evaluation whose path classifications stay consistent with its snapshots.
#[derive(Debug)]
pub struct Classification(Comparison);
impl Classification {
    /// Full selected base and head commit IDs, respectively.
    pub fn commits(&self) -> (&str, &str) {
        (&self.0.base.commit, &self.0.head.commit)
    }
    /// Changed paths in alphabetical order, with every applicable reason and version.
    pub fn paths(&self) -> impl Iterator<Item = PathClassification<'_>> {
        self.0.base.changes_to(&self.0.head)
    }
    /// Whether any changed path requires human review; this records no approval.
    pub fn requires_review(&self) -> bool {
        self.paths().any(|path| path.status() == Status::Review)
    }
}
/// How a path changed between the selected commits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeType {
    /// Exists only in the head commit.
    New,
    /// Changed content or file type.
    Modified,
    /// Exists only in the base commit.
    Deleted,
}
/// Whether review is needed; autonomy proves neither correctness nor contract satisfaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    /// No detected reason for human review.
    Autonomous,
    /// Human review is required.
    Review,
}
/// A review reason, including its supplying full commit ID where relevant.
#[derive(Debug, Eq, PartialEq)]
pub enum Reason {
    /// Newly created file, without a base version.
    NewFile,
    /// Existing version unassigned at this selected commit.
    Unassigned(String),
}
/// Component membership of an existing file version.
#[derive(Clone, Copy, Debug)]
pub struct PathDetails<'a> {
    commit: &'a str,
}
impl PathDetails<'_> {
    /// Assigned component; absent without component policy.
    pub fn component(&self) -> Option<&str> {
        None
    }
    fn reason(&self) -> Reason {
        Reason::Unassigned(self.commit.into())
    }
}
/// A changed path whose kind, status and reasons derive from its existing versions.
#[derive(Debug)]
pub struct PathClassification<'a> {
    path: &'a str,
    before: Option<PathDetails<'a>>,
    after: Option<PathDetails<'a>>,
}
impl PathClassification<'_> {
    /// Repository-relative changed path.
    pub fn path(&self) -> &str {
        self.path
    }
    /// Kind of committed change.
    pub fn change_type(&self) -> ChangeType {
        match (self.before, self.after) {
            (None, _) => ChangeType::New,
            (_, None) => ChangeType::Deleted,
            _ => ChangeType::Modified,
        }
    }
    /// Human review status derived from all applicable reasons.
    pub fn status(&self) -> Status {
        if self.reasons().next().is_none() {
            Status::Autonomous
        } else {
            Status::Review
        }
    }
    /// Every reason, ordered new-file first, then base and head unassigned versions.
    pub fn reasons(&self) -> impl Iterator<Item = Reason> + '_ {
        let new = self.before.is_none().then_some(Reason::NewFile);
        new.into_iter().chain(
            self.before
                .iter()
                .chain(self.after.iter())
                .map(PathDetails::reason),
        )
    }
    /// Base version details, absent for a new file.
    pub fn before(&self) -> Option<PathDetails<'_>> {
        self.before
    }
    /// Head version details, absent for a deleted file.
    pub fn after(&self) -> Option<PathDetails<'_>> {
        self.after
    }
}
