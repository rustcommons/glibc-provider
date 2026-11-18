#!/usr/bin/env python3
"""Refresh the Replay workflow's image digests and propose the change.

`resolve` looks up the current digest of each image tag in
qualification/replay-images.json on Docker Hub and rewrites the file.
`propose` commits a resolved file to a branch through the GitHub API and opens a
pull request, or, where Actions may not create pull requests, an issue with a
compare link. It accepts only digest changes. Stdlib only; never merges.
"""

import argparse
import base64
import json
import os
from pathlib import Path
import re
import urllib.error
import urllib.request

IMAGES = "qualification/replay-images.json"
BRANCH = "automation/replay-image-digests"
TITLE = "Refresh Replay image digests"
PINNED = re.compile(r"^([a-z0-9][a-z0-9._-]*):([A-Za-z0-9._-]+)@(sha256:[0-9a-f]{64})$")
ACCEPT = ", ".join([
    "application/vnd.oci.image.index.v1+json",
    "application/vnd.docker.distribution.manifest.list.v2+json",
    "application/vnd.oci.image.manifest.v1+json",
    "application/vnd.docker.distribution.manifest.v2+json",
])


def parse(text):
    entries = json.loads(text)
    if not isinstance(entries, list) or not entries:
        raise SystemExit(f"{IMAGES}: expected a non-empty list")
    for entry in entries:
        if not PINNED.match(entry.get("image", "")):
            raise SystemExit(f"{IMAGES}: {entry.get('name')}: image must be name:tag@sha256:...")
    return entries


def render(entries):
    return "[\n" + ",\n".join("  " + json.dumps(e) for e in entries) + "\n]\n"


def docker_hub_digest(name, tag):
    """The digest Docker Hub serves for an official image tag (multi-arch index)."""
    scope = f"repository:library/{name}:pull"
    with urllib.request.urlopen(
        f"https://auth.docker.io/token?service=registry.docker.io&scope={scope}", timeout=30
    ) as response:
        token = json.load(response)["token"]
    request = urllib.request.Request(
        f"https://registry-1.docker.io/v2/library/{name}/manifests/{tag}",
        method="HEAD",
        headers={"Authorization": f"Bearer {token}", "Accept": ACCEPT},
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        digest = response.headers.get("Docker-Content-Digest", "")
    if not re.fullmatch(r"sha256:[0-9a-f]{64}", digest):
        raise SystemExit(f"{name}:{tag}: registry returned no digest")
    return digest


def refresh(entries, lookup=docker_hub_digest):
    """New entries with each tag's current digest, and the changed images."""
    updated, changes = [], []
    for entry in entries:
        name, tag, old = PINNED.match(entry["image"]).groups()
        new = lookup(name, tag)
        updated.append({**entry, "image": f"{name}:{tag}@{new}"})
        if new != old:
            changes.append((entry["name"], f"{name}:{tag}", old, new))
    return updated, changes


def only_digests_changed(old, new):
    """True when the files differ in nothing but image digests."""
    strip = lambda entries: [{**e, "image": e["image"].split("@")[0]} for e in entries]
    return strip(old) == strip(new)


def summary(changes):
    if not changes:
        return "Every Replay image tag still has its pinned digest.\n"
    lines = [
        "The Replay images below have new digests: the same tags now hold different",
        "compilers or packages. Review the Replay run on this change before merging;",
        "it records each image's exact GCC and binutils versions.",
        "",
        "| Job | Tag | Old digest | New digest |",
        "| --- | --- | --- | --- |",
    ]
    lines += [f"| {n} | `{t}` | `{o[7:19]}` | `{d[7:19]}` |" for n, t, o, d in changes]
    return "\n".join(lines) + "\n"


class GitHub:
    def __init__(self, repo, token):
        self.base, self.token = f"https://api.github.com/repos/{repo}", token

    def __call__(self, method, path, body=None):
        request = urllib.request.Request(
            self.base + path,
            method=method,
            data=None if body is None else json.dumps(body).encode(),
            headers={
                "Authorization": f"Bearer {self.token}",
                "Accept": "application/vnd.github+json",
                "X-GitHub-Api-Version": "2022-11-28",
            },
        )
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.load(response)


def propose(api, base_sha, new_text, body, repo):
    """Commits `new_text` on BRANCH from `base_sha` and opens or updates the PR."""
    current = api("GET", f"/contents/{IMAGES}?ref={base_sha}")
    old = parse(base64.b64decode(current["content"]).decode())
    new = parse(new_text)
    if not only_digests_changed(old, new):
        raise SystemExit("refusing: the resolved file changes more than image digests")
    if old == new:
        print("No digest changed.")
        return None
    base = api("GET", f"/git/commits/{base_sha}")
    blob = api("POST", "/git/blobs", {"content": new_text, "encoding": "utf-8"})
    tree = api("POST", "/git/trees", {
        "base_tree": base["tree"]["sha"],
        "tree": [{"path": IMAGES, "mode": "100644", "type": "blob", "sha": blob["sha"]}],
    })
    commit = api("POST", "/git/commits", {
        "message": f"{TITLE}\n\n{body}", "tree": tree["sha"], "parents": [base_sha],
    })
    try:
        # The branch belongs to this workflow; each run replaces it.
        api("PATCH", f"/git/refs/heads/{BRANCH}", {"sha": commit["sha"], "force": True})
    except urllib.error.HTTPError as error:
        if error.code != 422:
            raise
        api("POST", "/git/refs", {"ref": f"refs/heads/{BRANCH}", "sha": commit["sha"]})
    owner = repo.split("/")[0]
    open_prs = api("GET", f"/pulls?state=open&head={owner}:{BRANCH}")
    if open_prs:
        api("PATCH", f"/pulls/{open_prs[0]['number']}", {"body": body})
        return open_prs[0]["html_url"]
    try:
        pr = api("POST", "/pulls", {"title": TITLE, "head": BRANCH, "base": "main", "body": body})
        return pr["html_url"]
    except urllib.error.HTTPError as error:
        if error.code != 403:
            raise
    # Actions may not create pull requests here: ask a maintainer to open it.
    compare = f"https://github.com/{repo}/compare/main...{BRANCH}?expand=1"
    issue_body = f"Open the pull request: {compare}\n\n{body}"
    issues = [
        i for i in api("GET", "/issues?state=open&creator=github-actions%5Bbot%5D")
        if i["title"] == TITLE and "pull_request" not in i
    ]
    if issues:
        api("PATCH", f"/issues/{issues[0]['number']}", {"body": issue_body})
        return issues[0]["html_url"]
    return api("POST", "/issues", {"title": TITLE, "body": issue_body})["html_url"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    resolve = sub.add_parser("resolve")
    resolve.add_argument("--summary", required=True, type=Path)
    prop = sub.add_parser("propose")
    prop.add_argument("--resolved", required=True, type=Path)
    prop.add_argument("--summary", required=True, type=Path)
    args = parser.parse_args()
    if args.command == "resolve":
        updated, changes = refresh(parse(Path(IMAGES).read_text()))
        Path(IMAGES).write_text(render(updated))
        args.summary.write_text(summary(changes))
        print(summary(changes))
    else:
        api = GitHub(os.environ["GITHUB_REPOSITORY"], os.environ["GH_TOKEN"])
        url = propose(
            api, os.environ["BASE_SHA"], args.resolved.read_text(), args.summary.read_text(),
            os.environ["GITHUB_REPOSITORY"],
        )
        if url:
            print(f"Proposed: {url}")


if __name__ == "__main__":
    main()
