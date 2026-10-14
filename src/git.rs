//! Collect complete committed trees and changes using local Git, without network operations.
use crate::{Change, ChangeType, CollectedComparison, CollectedSnapshot};
use std::path::{Path, PathBuf};
use std::process::Command;

fn run(root: &Path, arguments: &[&str], input: &str) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot read {input:?} in {}: {error}", root.display()))?;
    if !output.status.success() {
        return Err(format!(
            "Git operation failed for {input:?}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}
fn text(bytes: Vec<u8>, input: &str) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|_| format!("non-UTF-8 Git output for {input:?}"))
}
fn resolve(root: &Path, reference: &str) -> Result<String, String> {
    let commit = format!("{reference}^{{commit}}");
    let args = ["rev-parse", "--verify", "--end-of-options", &commit];
    let output = run(root, &args, reference)?;
    Ok(text(output, reference)?.trim().into())
}
fn snapshot(root: &Path, commit: String) -> Result<CollectedSnapshot, String> {
    let args = ["ls-tree", "-r", "--name-only", "-z", &commit];
    let output = run(root, &args, &commit)?;
    let files: Vec<String> = text(output, &commit)?
        .split_terminator('\0')
        .map(str::to_owned)
        .collect();
    let policy = if files.iter().any(|path| path == "cabco.yaml") {
        let object = format!("{commit}:cabco.yaml");
        Some(run(root, &["show", &object], &commit)?)
    } else {
        None
    };
    Ok(CollectedSnapshot {
        commit,
        policy,
        files,
    })
}
fn changes(root: &Path, base: &str, head: &str) -> Result<Vec<Change>, String> {
    let input = format!("{base}..{head}");
    let args = [
        "diff",
        "--no-ext-diff",
        "--no-renames",
        "--name-status",
        "-z",
        base,
        head,
        "--",
    ];
    let output = text(run(root, &args, &input)?, &input)?;
    let fields: Vec<_> = output.split_terminator('\0').collect();
    let (pairs, remainder) = fields.as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(format!("malformed Git changes for {input:?}"));
    }
    pairs
        .iter()
        .map(|pair| {
            let path = pair[1].to_owned();
            let change_type = match pair[0] {
                "A" => ChangeType::New,
                "D" => ChangeType::Deleted,
                "M" | "T" => ChangeType::Modified,
                status => {
                    return Err(format!(
                        "unsupported Git change {status:?} for {path:?} in {input}"
                    ));
                }
            };
            Ok(Change { path, change_type })
        })
        .collect()
}
/// Collect two selected commits from a repository root or subdirectory.
///
/// References must name commits. Returns full IDs, complete tracked path lists,
/// policy bytes, and a direct tree comparison; ignores working-tree edits.
/// Moves are deletion plus creation. Errors identify unreadable inputs, including
/// paths that cannot be represented as UTF-8. Never fetches or pushes.
pub fn collect(directory: &Path, base: &str, head: &str) -> Result<CollectedComparison, String> {
    let output = run(directory, &["rev-parse", "--show-toplevel"], "repository")?;
    let root = PathBuf::from(text(output, "repository")?.trim_end_matches('\n'));
    let base = resolve(&root, base)?;
    let head = resolve(&root, head)?;
    Ok(CollectedComparison {
        changes: changes(&root, &base, &head)?,
        before: snapshot(&root, base)?,
        after: snapshot(&root, head)?,
    })
}
