//! Exercise the reusable comparison core without Git or filesystem access.
use cabco::{ChangeType::*, Comparison, Reason, Snapshot, Status};

const BASE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HEAD: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn complete_snapshots_produce_consistent_changes_and_all_review_reasons() {
    let comparison = Comparison::new(
        snapshot(
            BASE,
            &[
                ("edit", "old"),
                ("gone", "deleted"),
                ("mode", "plain:same"),
                ("old", "moved"),
                ("same", "same"),
            ],
        ),
        snapshot(
            HEAD,
            &[
                ("copy", "same"),
                ("edit", "new"),
                ("mode", "executable:same"),
                ("new", "moved"),
                ("same", "same"),
            ],
        ),
    );
    assert_eq!(
        comparison.snapshots().0.paths().collect::<Vec<_>>(),
        ["edit", "gone", "mode", "old", "same"]
    );
    assert_eq!(
        comparison.snapshots().1.paths().collect::<Vec<_>>(),
        ["copy", "edit", "mode", "new", "same"]
    );
    assert!(comparison.snapshots().0.policy().is_none());
    let result = comparison.evaluate().unwrap();
    assert_eq!(result.commits(), (BASE, HEAD));
    assert!(result.requires_review());
    let paths: Vec<_> = result.paths().collect();
    let expected = [
        (
            "copy",
            New,
            false,
            true,
            vec![Reason::NewFile, unassigned(HEAD)],
        ),
        (
            "edit",
            Modified,
            true,
            true,
            vec![unassigned(BASE), unassigned(HEAD)],
        ),
        ("gone", Deleted, true, false, vec![unassigned(BASE)]),
        (
            "mode",
            Modified,
            true,
            true,
            vec![unassigned(BASE), unassigned(HEAD)],
        ),
        (
            "new",
            New,
            false,
            true,
            vec![Reason::NewFile, unassigned(HEAD)],
        ),
        ("old", Deleted, true, false, vec![unassigned(BASE)]),
    ];
    assert_eq!(paths.len(), expected.len());
    for (path, (name, kind, before, after, reasons)) in paths.iter().zip(expected) {
        assert_eq!(path.path(), name);
        assert_eq!(path.change_type(), kind);
        assert_eq!(path.status(), Status::Review);
        assert_eq!(path.reasons().collect::<Vec<_>>(), reasons);
        assert_eq!(
            (path.before().is_some(), path.after().is_some()),
            (before, after)
        );
        for version in path.before().iter().chain(path.after().iter()) {
            assert_eq!(version.component(), None);
        }
    }
}

#[test]
fn invalid_snapshot_states_are_rejected_without_io() {
    for commit in [
        String::new(),
        "HEAD".into(),
        "a".repeat(39),
        "a".repeat(41),
        "g".repeat(40),
    ] {
        assert!(Snapshot::new(commit, Default::default(), None).is_err());
    }
    for (path, identity, policy) in [
        ("", "one", None),
        ("/file", "one", None),
        ("./file", "one", None),
        ("dir/../file", "one", None),
        ("dir//file", "one", None),
        ("file\0", "one", None),
        ("file", "", None),
        ("cabco.yaml", "one", None),
        ("file", "one", Some(vec![])),
    ] {
        let files = [(path.into(), identity.into())].into();
        let error = Snapshot::new(BASE.into(), files, policy).unwrap_err();
        assert!(error.contains(BASE), "{error}");
    }
    let files = [
        ("dir".into(), "one".into()),
        ("dir/file".into(), "two".into()),
    ]
    .into();
    assert!(Snapshot::new(BASE.into(), files, None).is_err());
}

#[test]
fn equal_trees_have_no_changed_paths_even_with_different_commit_ids() {
    for files in [&[][..], &[("dir with spaces/naïve", "same")][..]] {
        for head in [BASE, HEAD] {
            let result = Comparison::new(snapshot(BASE, files), snapshot(head, files))
                .evaluate()
                .unwrap();
            assert_eq!(result.commits(), (BASE, head));
            assert_eq!(result.paths().count(), 0);
            assert!(!result.requires_review());
        }
    }
}

#[test]
fn policy_bytes_are_inspectable_and_rejected_on_either_side_even_for_equal_snapshots() {
    let bytes = b"not parsed: [\xff";
    let selected = |commit: &str, policy| {
        if policy {
            Snapshot::new(
                commit.into(),
                [("cabco.yaml".into(), "policy".into())].into(),
                Some(bytes.to_vec()),
            )
            .unwrap()
        } else {
            snapshot(commit, &[])
        }
    };
    for (base_policy, head_policy, head) in
        [(true, false, HEAD), (false, true, HEAD), (true, true, BASE)]
    {
        let comparison = Comparison::new(selected(BASE, base_policy), selected(head, head_policy));
        let (before, after) = comparison.snapshots();
        assert_eq!(before.policy(), base_policy.then_some(bytes.as_slice()));
        assert_eq!(after.policy(), head_policy.then_some(bytes.as_slice()));
        let error = comparison.evaluate().unwrap_err();
        let supplying_commit = if base_policy { BASE } else { head };
        assert!(
            error.contains("unsupported") && error.contains(supplying_commit),
            "{error}"
        );
    }
}

fn snapshot(commit: &str, files: &[(&str, &str)]) -> Snapshot {
    let files = files
        .iter()
        .map(|(path, identity)| ((*path).into(), (*identity).into()))
        .collect();
    Snapshot::new(commit.into(), files, None).unwrap()
}
fn unassigned(commit: &str) -> Reason {
    Reason::Unassigned(commit.into())
}
