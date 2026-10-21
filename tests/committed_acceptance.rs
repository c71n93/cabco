//! Acceptance tests through the reusable core, Git collector, and actual CLI.
use cabco::{ChangeType::*, Reason, Status, git as collector};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

#[test]
fn divergent_committed_trees_supply_complete_collection_core_reasons_and_cli_output() {
    let repo = repository();
    let root = repo.path();
    write(root, "src/same", "same");
    write(root, "src/mode", "unchanged content");
    let common = commit(root);
    write(root, "src/edit", "base");
    write(root, "src/old", "move");
    let base = commit(root);
    git(root, &["branch", "base-selected"]);
    git(root, &["checkout", "--detach", &common]);
    write(root, "src/edit", "head");
    write(root, "src/new", "move");
    write(root, "src/copy", "same");
    git(root, &["add", "."]);
    git(root, &["update-index", "--chmod=+x", "src/mode"]);
    git(root, &["commit", "-qm", "head with mode change"]);
    let head = git(root, &["rev-parse", "HEAD"]);
    write(root, "src/edit", "dirty");
    write(root, "cabco.yaml", "untracked policy");
    let nested = root.join("src");
    let input = collector::collect(&nested, "base-selected", "HEAD").unwrap();
    assert_eq!(
        input.snapshots().0.paths().collect::<Vec<_>>(),
        ["src/edit", "src/mode", "src/old", "src/same"]
    );
    assert_eq!(
        input.snapshots().1.paths().collect::<Vec<_>>(),
        ["src/copy", "src/edit", "src/mode", "src/new", "src/same"]
    );
    assert!(input.snapshots().0.policy().is_none() && input.snapshots().1.policy().is_none());
    let result = input.evaluate().unwrap();
    assert_eq!(result.commits(), (base.as_str(), head.as_str()));
    assert!(result.requires_review());
    let unassigned = |commit: &str| Reason::Unassigned(commit.into());
    let expected = [
        vec![Reason::NewFile, unassigned(&head)],
        vec![unassigned(&base), unassigned(&head)],
        vec![unassigned(&base), unassigned(&head)],
        vec![Reason::NewFile, unassigned(&head)],
        vec![unassigned(&base)],
    ];
    let kinds: Vec<_> = result.paths().map(|entry| entry.change_type()).collect();
    assert_eq!(kinds, [New, Modified, Modified, New, Deleted]);
    let versions: Vec<_> = result
        .paths()
        .map(|entry| (entry.before().is_some(), entry.after().is_some()))
        .collect();
    assert_eq!(
        versions,
        [
            (false, true),
            (true, true),
            (true, true),
            (false, true),
            (true, false)
        ]
    );
    for (entry, reasons) in result.paths().zip(expected) {
        assert_eq!(entry.status(), Status::Review);
        assert_eq!(entry.reasons().collect::<Vec<_>>(), reasons);
        for details in entry.before().iter().chain(entry.after().iter()) {
            assert_eq!(details.component(), None);
        }
    }
    let output = cabco(&nested, "base-selected", "HEAD");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "REVIEW (5)\nsrc/copy\nsrc/edit\nsrc/mode\nsrc/new\nsrc/old\n"
    );
    let output = cabco(root, &head, &head);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "AUTONOMOUS — no changed files\n"
    );
}

#[test]
fn policy_contents_are_collected_and_rejected_on_either_side_even_without_changes() {
    let repo = repository();
    let root = repo.path();
    write(root, "file", "one");
    let plain = commit(root);
    write(root, "cabco.yaml", "not parsed: [");
    let policy = commit(root);
    for (base, head) in [(&plain, &policy), (&policy, &plain), (&policy, &policy)] {
        let input = collector::collect(root, base, head).unwrap();
        let snapshot = if base == &policy {
            input.snapshots().0
        } else {
            input.snapshots().1
        };
        assert_eq!(snapshot.policy().unwrap(), b"not parsed: [");
        let error = input.evaluate().unwrap_err();
        assert!(
            error.contains(&policy) && error.contains("unsupported"),
            "{error}"
        );
        let output = cabco(root, base, head);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn collection_and_argument_failures_identify_inputs_without_printing_a_verdict() {
    let repo = repository();
    write(repo.path(), "file", "one");
    let valid = commit(repo.path());
    for (base, head, input) in [
        ("missing-base", valid.as_str(), "missing-base"),
        (valid.as_str(), "missing-head", "missing-head"),
    ] {
        let error = collector::collect(repo.path(), base, head).unwrap_err();
        assert!(error.contains(input), "{error}");
        let output = cabco(repo.path(), base, head);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr).unwrap().contains(input));
    }
    let outside = tempfile::tempdir().unwrap();
    let error = collector::collect(outside.path(), &valid, &valid).unwrap_err();
    assert!(error.contains("repository"), "{error}");
    for arguments in [
        vec!["check"],
        vec!["check", "--base", &valid],
        vec!["check", "--head", &valid],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_cabco"))
            .args(arguments)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

fn write(directory: &Path, path: &str, content: &str) {
    let destination = directory.join(path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(destination, content).unwrap();
}
fn commit(directory: &Path) -> String {
    git(directory, &["add", "."]);
    git(directory, &["commit", "-qm", "snapshot"]);
    git(directory, &["rev-parse", "HEAD"])
}
fn repository() -> tempfile::TempDir {
    let repo = tempfile::tempdir().unwrap();
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.email", "test@example.com"]);
    git(repo.path(), &["config", "user.name", "CABCO Test"]);
    repo
}
fn cabco(directory: &Path, base: &str, head: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cabco"))
        .current_dir(directory)
        .args(["check", "--base", base, "--head", head])
        .output()
        .unwrap()
}
fn git(directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(directory)
        .args(arguments)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    String::from_utf8(output.stdout).unwrap().trim().into()
}
