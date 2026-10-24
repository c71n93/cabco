//! Collect complete committed trees and changes using local Git, without network operations.
use crate::{Comparison, Snapshot};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Collect two committed trees directly, with full IDs, complete file lists, and policy bytes.
/// References must name commits; accepts a repository root or subdirectory.
/// Ignores working-tree edits; moves are deletion plus creation. Never fetches or pushes.
///
/// # Errors
/// Identifies unreadable inputs, including invalid refs and non-UTF-8 paths.
pub fn collect(directory: &Path, base: &str, head: &str) -> Result<Comparison, String> {
    let output = run(directory, &["rev-parse", "--show-toplevel"], "repository")?;
    let root = PathBuf::from(text(output, "repository")?.trim_end_matches('\n'));
    let base = resolve(&root, base)?;
    let head = resolve(&root, head)?;
    Ok(Comparison::new(
        snapshot(&root, base)?,
        snapshot(&root, head)?,
    ))
}
fn resolve(root: &Path, reference: &str) -> Result<String, String> {
    let commit = format!("{reference}^{{commit}}");
    let args = ["rev-parse", "--verify", "--end-of-options", &commit];
    let output = run(root, &args, reference)?;
    Ok(text(output, reference)?.trim().into())
}
fn snapshot(root: &Path, commit: String) -> Result<Snapshot, String> {
    let args = ["ls-tree", "-r", "-z", &commit];
    let output = run(root, &args, &commit)?;
    let files: BTreeMap<String, String> = text(output, &commit)?
        .split_terminator('\0')
        .map(|entry| {
            let (identity, path) = entry
                .split_once('\t')
                .ok_or_else(|| format!("malformed Git tree at {commit}"))?;
            Ok((path.into(), identity.into()))
        })
        .collect::<Result<_, String>>()?;
    let policy = if files.contains_key("cabco.yaml") {
        let object = format!("{commit}:cabco.yaml");
        Some(run(root, &["show", &object], &commit)?)
    } else {
        None
    };
    Snapshot::new(commit, files, policy)
}

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
