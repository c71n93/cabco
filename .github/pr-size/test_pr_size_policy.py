import re
import unittest
from pathlib import Path


WORKFLOW = Path(__file__).parents[1] / "workflows" / "pr-size.yml"
CONTRIBUTING = Path(__file__).parents[2] / "CONTRIBUTING.md"
ACTION_REVISION = "19c335e7695ba922de938806dd129f0a9b992644"


class PrSizePolicyTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.workflow = WORKFLOW.read_text()

    def input_value(self, name: str) -> str:
        match = re.search(rf"^\s+{name}:\s*['\"]?([^'\"\n]+)", self.workflow, re.MULTILINE)
        self.assertIsNotNone(match, f"missing action input {name}")
        return match.group(1).strip()

    def test_allows_500_counted_lines_and_rejects_501(self) -> None:
        largest_non_failing_threshold = int(self.input_value("l_max_size"))

        self.assertFalse(500 >= largest_non_failing_threshold)
        self.assertTrue(501 >= largest_non_failing_threshold)
        self.assertEqual(self.input_value("fail_if_xl"), "true")

    def test_only_root_cargo_lock_is_excluded(self) -> None:
        ignored_files = self.input_value("files_to_ignore").split()
        changes = {
            "Cargo.lock": (400, 200),
            "crates/example/Cargo.lock": (1, 0),
            "tests/example.rs": (2, 3),
            "docs/policy.md": (4, 5),
            ".github/example.yml": (6, 7),
        }

        counted = sum(
            additions + deletions
            for filename, (additions, deletions) in changes.items()
            if filename not in ignored_files
        )

        self.assertEqual(ignored_files, ["Cargo.lock"])
        self.assertEqual(counted, 28)

    def test_lockfile_only_and_mixed_changes_use_additions_plus_deletions(self) -> None:
        ignored_files = self.input_value("files_to_ignore").split()

        def count(changes: list[tuple[str, int, int]]) -> int:
            return sum(
                additions + deletions
                for filename, additions, deletions in changes
                if filename not in ignored_files
            )

        self.assertEqual(count([("Cargo.lock", 700, 300)]), 0)
        self.assertEqual(
            count([("Cargo.lock", 700, 300), ("src/lib.rs", 250, 250)]),
            500,
        )
        self.assertEqual(
            count([("Cargo.lock", 700, 300), ("src/lib.rs", 250, 251)]),
            501,
        )

    def test_uses_pagination_capable_immutable_action_revision(self) -> None:
        self.assertIn(
            f"uses: CodelyTV/pr-size-labeler@{ACTION_REVISION}", self.workflow
        )
        self.assertNotRegex(self.workflow, r"CodelyTV/pr-size-labeler@v\d")

    def test_changes_after_the_first_100_files_can_make_the_pr_oversized(self) -> None:
        first_page = [(f"docs/page-{number}.md", 3, 2) for number in range(100)]
        second_page = [("src/lib.rs", 1, 0)]

        counted = sum(
            additions + deletions
            for _, additions, deletions in first_page + second_page
        )

        self.assertEqual(counted, 501)
        self.assertIn(
            f"uses: CodelyTV/pr-size-labeler@{ACTION_REVISION}", self.workflow
        )

    def test_handles_fork_prs_without_executing_pr_code(self) -> None:
        self.assertIn("pull_request_target:", self.workflow)
        for event in ("opened", "reopened", "synchronize"):
            self.assertRegex(self.workflow, rf"types:.*\b{event}\b")
        self.assertRegex(self.workflow, r"(?m)^\s+contents: read$")
        self.assertRegex(self.workflow, r"(?m)^\s+pull-requests: write$")
        self.assertNotIn("actions/checkout", self.workflow)
        self.assertNotRegex(self.workflow, r"(?m)^\s+run:")

    def test_exposes_one_uniquely_named_enforcement_check_and_clear_failure(self) -> None:
        self.assertEqual(
            len(re.findall(r"name: Enforce 500-line PR size limit", self.workflow)),
            1,
        )
        message = self.input_value("message_if_xl")
        self.assertIn("500 counted lines", message)
        self.assertIn("smaller, coherent changes", message)

    def test_contributor_policy_documents_ceiling_exclusion_and_agent_expectation(self) -> None:
        policy = " ".join(CONTRIBUTING.read_text().split())

        self.assertIn("500 changed lines", policy)
        self.assertIn("additions plus deletions", policy)
        self.assertIn("repository-root `Cargo.lock`", policy)
        self.assertIn("regardless of maintenance ownership", policy)
        self.assertIn("Agents must plan smaller, coherent implementation increments", policy)
        self.assertIn("relevant tests", policy)


if __name__ == "__main__":
    unittest.main()
