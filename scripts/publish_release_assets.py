"""Resume a draft release, validate every asset, and publish without replacing files."""

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile

from release_signing import verify


def gh(*args):
    return subprocess.check_output(["gh", *args])


def check_asset(asset, file, repo):
    data = gh("api", f"repos/{repo}/releases/assets/{asset['id']}",
              "-H", "Accept: application/octet-stream")
    if len(data) != asset["size"]:
        raise ValueError(f"Release asset size mismatch: {file.name}")
    if file.name == "SHA256SUMS.asc":
        # A rerun creates a new signature timestamp. Keep the original signature
        # if it authenticates the identical manifest with the pinned key.
        with tempfile.TemporaryDirectory() as temp:
            manifest = Path(temp) / "SHA256SUMS"
            manifest.write_bytes(file.with_name("SHA256SUMS").read_bytes())
            manifest.with_suffix(".asc").write_bytes(data)
            # Copy only the assets named by this repository's manifest.
            for asset_file in file.parent.iterdir():
                if asset_file.suffix in {".crate", ".json"}:
                    (Path(temp) / asset_file.name).write_bytes(asset_file.read_bytes())
            verify(manifest)
    elif (asset["size"] != file.stat().st_size
          or hashlib.sha256(data).digest() != hashlib.sha256(file.read_bytes()).digest()):
        raise ValueError(f"Release asset digest mismatch: {file.name}")


def publish(repo, version, dist):
    tag = "v" + version
    notes = Path(f"docs/releases/{tag}.md").read_text()
    title = next(line[2:] for line in notes.splitlines() if line.startswith("# "))
    files = [dist / f"glibc-provider-sources-{version}.crate",
             dist / f"glibc-provider-sources-{version}.cdx.json",
             dist / "SHA256SUMS", dist / "SHA256SUMS.asc",
             dist / f"glibc-provider-{version}.crates.intoto.jsonl"]
    for file in files:
        if not file.is_file():
            raise ValueError(f"Missing asset: {file.name}")
    # The releases list includes drafts, unlike tag lookup.
    releases = json.loads(gh("api", "--paginate", "--slurp", f"repos/{repo}/releases"))
    matches = [r for page in releases for r in page if r["tag_name"] == tag]
    if len(matches) > 1:
        raise ValueError("Multiple releases for tag")
    if not matches:
        with tempfile.TemporaryDirectory() as temp:
            body = Path(temp) / "notes.md"
            body.write_text(notes)
            gh("release", "create", tag, "--repo", repo, "--verify-tag", "--draft",
               "--title", title, "--notes-file", str(body))
        releases = json.loads(gh("api", "--paginate", "--slurp", f"repos/{repo}/releases"))
        matches = [r for page in releases for r in page if r["tag_name"] == tag]
    if len(matches) != 1:
        raise ValueError("Expected one release")
    release_id = matches[0]["id"]

    def inspect(allow_missing):
        release = json.loads(gh("api", f"repos/{repo}/releases/{release_id}"))
        if (release["tag_name"] != tag or release["name"] != title
                or release["body"] != notes or release["prerelease"]):
            raise ValueError("Release metadata differs from committed notes")
        assets = {a["name"]: a for a in release["assets"]}
        expected = {f.name for f in files}
        if set(assets) - expected:
            raise ValueError("Unexpected release assets")
        for file in files:
            if file.name in assets:
                check_asset(assets[file.name], file, repo)
            elif not (allow_missing and release["draft"]):
                raise ValueError(f"Missing published asset: {file.name}")
        return release, assets

    release, assets = inspect(True)
    if release["draft"]:
        for file in files:
            if file.name not in assets:
                gh("release", "upload", tag, str(file), "--repo", repo)
        release, _ = inspect(False)
        if not release["draft"]:
            raise ValueError("Release changed state during upload")
        gh("api", "--method", "PATCH", f"repos/{repo}/releases/{release_id}",
           "-F", "draft=false")
    release, _ = inspect(False)
    if release["draft"]:
        raise ValueError("Release is still a draft")


if __name__ == "__main__":
    publish(sys.argv[1], sys.argv[2], Path(sys.argv[3]))
