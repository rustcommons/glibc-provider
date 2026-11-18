"""Replay image refresh: digest-only changes, offline with a fake registry and API."""

import base64
import importlib.util
import io
from pathlib import Path
import unittest
import urllib.error

spec = importlib.util.spec_from_file_location(
    "digests", Path(__file__).with_name("replay-image-digests.py")
)
digests = importlib.util.module_from_spec(spec)
spec.loader.exec_module(digests)

OLD, NEW = "sha256:" + "a" * 64, "sha256:" + "b" * 64
ENTRIES = [
    {"name": "GCC 12", "image": f"gcc:12@{OLD}"},
    {"name": "Fedora", "image": f"fedora:43@{OLD}", "install": "dnf install -y gcc"},
]


class FakeGitHub:
    def __init__(self, text, pr_allowed=True, branch_exists=False):
        self.text, self.pr_allowed, self.branch_exists = text, pr_allowed, branch_exists
        self.calls = []

    def __call__(self, method, path, body=None):
        self.calls.append((method, path.split("?")[0], body))
        if path.startswith("/contents/"):
            return {"content": base64.b64encode(self.text.encode()).decode()}
        if method == "PATCH" and path.startswith("/git/refs/") and not self.branch_exists:
            raise urllib.error.HTTPError(path, 422, "missing", {}, io.BytesIO())
        if method == "POST" and path == "/pulls" and not self.pr_allowed:
            raise urllib.error.HTTPError(path, 403, "not permitted", {}, io.BytesIO())
        if method == "GET" and path.startswith(("/pulls", "/issues")):
            return []
        return {"sha": "c" * 40, "tree": {"sha": "d" * 40}, "html_url": "url"}


class DigestTests(unittest.TestCase):
    def test_refresh_reports_only_changed_tags(self):
        lookup = lambda name, tag: NEW if name == "gcc" else OLD
        updated, changes = digests.refresh(ENTRIES, lookup)
        self.assertEqual(updated[0]["image"], f"gcc:12@{NEW}")
        self.assertEqual(updated[1], ENTRIES[1])
        self.assertEqual([c[0] for c in changes], ["GCC 12"])
        self.assertIn("`gcc:12`", digests.summary(changes))
        self.assertEqual(digests.parse(digests.render(updated)), updated)

    def test_rejects_unpinned_images_and_other_changes(self):
        with self.assertRaises(SystemExit):
            digests.parse('[{"name": "x", "image": "gcc:12"}]')
        changed = [dict(ENTRIES[0]), {**ENTRIES[1], "install": "curl x | sh"}]
        self.assertFalse(digests.only_digests_changed(ENTRIES, changed))
        api = FakeGitHub(digests.render(ENTRIES))
        with self.assertRaises(SystemExit):
            digests.propose(api, "e" * 40, digests.render(changed), "body", "o/r")
        self.assertFalse([c for c in api.calls if c[0] != "GET"], "nothing written")

    def test_no_change_writes_nothing(self):
        api = FakeGitHub(digests.render(ENTRIES))
        self.assertIsNone(digests.propose(api, "e" * 40, digests.render(ENTRIES), "b", "o/r"))
        self.assertFalse([c for c in api.calls if c[0] != "GET"])

    def test_opens_a_pull_request_on_a_new_branch(self):
        updated, _ = digests.refresh(ENTRIES, lambda n, t: NEW)
        api = FakeGitHub(digests.render(ENTRIES))
        self.assertEqual(digests.propose(api, "e" * 40, digests.render(updated), "b", "o/r"), "url")
        writes = [(m, p) for m, p, _ in api.calls if m != "GET"]
        self.assertIn(("POST", "/git/refs"), writes)
        self.assertEqual(writes[-1], ("POST", "/pulls"))
        commit = next(b for m, p, b in api.calls if p == "/git/commits")
        self.assertEqual(commit["parents"], ["e" * 40])
        tree = next(b for m, p, b in api.calls if p == "/git/trees")
        self.assertEqual([t["path"] for t in tree["tree"]], [digests.IMAGES])
        self.assertFalse(any("merge" in p for _, p, _ in api.calls), "never merges")

    def test_falls_back_to_an_issue_when_actions_may_not_open_prs(self):
        updated, _ = digests.refresh(ENTRIES, lambda n, t: NEW)
        api = FakeGitHub(digests.render(ENTRIES), pr_allowed=False, branch_exists=True)
        digests.propose(api, "e" * 40, digests.render(updated), "b", "o/r")
        method, path, body = api.calls[-1]
        self.assertEqual((method, path), ("POST", "/issues"))
        self.assertIn("compare/main...automation/replay-image-digests", body["body"])


if __name__ == "__main__":
    unittest.main()
