#!/usr/bin/env python3
"""Compare the packaged crate with crates.io metadata and the official archive."""

import hashlib
import json
from pathlib import Path
import time
import tomllib
import urllib.error
import urllib.request


def verify(package, archive, attempts=12, delay=10):
    name, version = package["name"], package["version"]
    expected = hashlib.sha256(Path(archive).read_bytes()).hexdigest()
    # Publication/CDN visibility can lag. A visible mismatch fails immediately.
    for attempt in range(attempts):
        try:
            request = urllib.request.Request(
                f"https://crates.io/api/v1/crates/{name}/{version}",
                headers={"User-Agent": "rustcommons-release-checksum-check"},
            )
            with urllib.request.urlopen(request, timeout=30) as response:
                record = json.load(response)["version"]
            if record["yanked"] or record["checksum"] != expected:
                raise SystemExit("Registry checksum mismatch or version is yanked")
            url = f"https://static.crates.io/crates/{name}/{name}-{version}.crate"
            with urllib.request.urlopen(url, timeout=30) as response:
                actual = hashlib.sha256(response.read()).hexdigest()
            if actual != expected:
                raise SystemExit("Official archive checksum mismatch")
            print(f"Verified {name} {version}: {actual}")
            return
        except urllib.error.HTTPError as error:
            if error.code not in (404, 429, 500, 502, 503, 504):
                raise
            if attempt == attempts - 1:
                raise
            time.sleep(delay)
    raise SystemExit("Publication did not become visible")


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, help="the attested .crate file "
                        "(default: target/package/<name>-<version>.crate)")
    args = parser.parse_args()
    with open("Cargo.toml", "rb") as manifest:
        package = tomllib.load(manifest)["package"]
    default = Path("target/package") / f"{package['name']}-{package['version']}.crate"
    verify(package, args.archive or default)
