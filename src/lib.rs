//! Evaluate committed file changes independently of Git collection and CLI output.

pub mod git;

/// How a path changed between the selected commits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChangeType {
    /// A file exists only in the head commit.
    New,
    /// An existing file changed content or type.
    Modified,
    /// A file exists only in the base commit.
    Deleted,
}
/// A changed path collected from two committed trees.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Change {
    /// Repository-relative path.
    pub path: String,
    /// Kind of committed change.
    pub change_type: ChangeType,
}
/// Complete file membership and policy contents at one selected commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectedSnapshot {
    /// Resolved full commit ID.
    pub commit: String,
    /// Contents of root `cabco.yaml`, when present.
    pub policy: Option<Vec<u8>>,
    /// All tracked paths, including unchanged files.
    pub files: Vec<String>,
}
/// Git-independent inputs for core evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectedComparison {
    /// Base commit snapshot.
    pub before: CollectedSnapshot,
    /// Head commit snapshot.
    pub after: CollectedSnapshot,
    /// Changes between these trees, with moves represented as deletion and creation.
    pub changes: Vec<Change>,
}
/// Whether a change requires human review; this records no approval.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    /// No detected reason for human review; proves neither correctness nor contract satisfaction.
    Autonomous,
    /// Human review is required.
    Review,
}
/// One review reason, with its supplying commit when relevant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Reason {
    /// A file was newly created; there is no base version.
    NewFile,
    /// An existing file version is unassigned at this full selected commit ID.
    Unassigned(String),
}
/// Component membership of an existing file version.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathDetails {
    /// Assigned component; absent when no component policy exists.
    pub component: Option<String>,
}
/// Evaluation of one changed path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PathClassification {
    /// Repository-relative path.
    pub path: String,
    /// Kind of committed change.
    pub change_type: ChangeType,
    /// Review status.
    pub status: Status,
    /// Every applicable review reason.
    pub reasons: Vec<Reason>,
    /// Base version details; absent for new files.
    pub before: Option<PathDetails>,
    /// Head version details; absent for deleted files.
    pub after: Option<PathDetails>,
}
/// Reusable evaluation result for the two selected commits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Classification {
    /// Resolved base commit ID.
    pub base_commit: String,
    /// Resolved head commit ID.
    pub head_commit: String,
    /// Every changed path and its evaluation.
    pub paths: Vec<PathClassification>,
}
impl Classification {
    /// Whether at least one changed path requires human review.
    pub fn requires_review(&self) -> bool {
        self.paths
            .iter()
            .any(|entry| entry.status == Status::Review)
    }
}
/// Evaluate a collected comparison with no component policy in either snapshot.
///
/// Returns an error identifying the selected commit if a policy is present.
/// Inputs must describe consistent committed trees and their changes.
pub fn evaluate(input: CollectedComparison) -> Result<Classification, String> {
    for snapshot in [&input.before, &input.after] {
        if snapshot.policy.is_some() {
            let commit = &snapshot.commit;
            return Err(format!(
                "component policy cabco.yaml at {commit} is unsupported"
            ));
        }
    }
    let mut paths = Vec::new();
    for change in input.changes {
        let version = |exists: bool| exists.then_some(PathDetails { component: None });
        let before = version(change.change_type != ChangeType::New);
        let after = version(change.change_type != ChangeType::Deleted);
        let mut reasons = Vec::new();
        if change.change_type == ChangeType::New {
            reasons.push(Reason::NewFile);
        }
        for (details, commit) in [
            (&before, &input.before.commit),
            (&after, &input.after.commit),
        ] {
            if details.is_some() {
                reasons.push(Reason::Unassigned(commit.clone()));
            }
        }
        paths.push(PathClassification {
            path: change.path,
            change_type: change.change_type,
            status: Status::Review,
            reasons,
            before,
            after,
        });
    }
    Ok(Classification {
        base_commit: input.before.commit,
        head_commit: input.after.commit,
        paths,
    })
}
