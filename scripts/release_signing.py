"""OpenPGP release signatures. Production entry points always enforce the owner pin."""

import argparse
from contextlib import contextmanager
import os
from pathlib import Path
import shutil
import subprocess

PRIMARY_FINGERPRINT = "B281C3010A76A11737B4D066C7396F6EA7647CC6"
SIGNING_SUBKEY = "248DFD639B75D1EFF2A87289D7C7DE7BBCCF35EC"
PUBLIC_KEY = Path(__file__).resolve().parents[1] / ".github/release-signing-key.asc"


@contextmanager
def gpg_home():
    home = subprocess.check_output(["mktemp", "-d"], text=True).strip()
    os.chmod(home, 0o700)
    try:
        yield home
    finally:
        try:
            subprocess.run(["gpgconf", "--homedir", home, "--kill", "gpg-agent"],
                           check=True)
        finally:
            shutil.rmtree(home)


def gpg(home, *args, data=None):
    # Do not pass signing secrets through the child environment or diagnostics.
    env = {k: v for k, v in os.environ.items()
           if k not in {"RELEASE_SIGNING_KEY", "RELEASE_SIGNING_PASSPHRASE"}}
    return subprocess.run(["gpg", "--homedir", home, "--batch", *args], input=data,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                          check=True, env=env).stdout


def primary_fingerprint(home):
    records = gpg(home, "--with-colons", "--list-keys").decode().splitlines()
    primaries = []
    want_primary = False
    for record in records:
        fields = record.split(":")
        if fields[0] == "pub":
            want_primary = True
        elif fields[0] == "fpr" and want_primary:
            primaries.append(fields[9])
            want_primary = False
    if len(primaries) != 1:
        raise ValueError("Expected exactly one primary public key")
    return primaries[0]


def require_validsig(status, expected):
    signatures = [line.split()[2:] for line in status.decode().splitlines()
                  if line.startswith("[GNUPG:] VALIDSIG ")]
    # VALIDSIG: signing fingerprint, date, timestamp, expiry, version,
    # reserved, pubkey algo, hash algo, signature class, primary fingerprint.
    if len(signatures) != 1 or len(signatures[0]) != 10:
        raise ValueError("Expected one complete VALIDSIG record")
    if signatures[0][-1] != expected:
        raise ValueError("Signature primary fingerprint does not match pin")


def _verify(manifest, public_key, expected):
    with gpg_home() as home:
        gpg(home, "--import", data=public_key)
        if primary_fingerprint(home) != expected:
            raise ValueError("Public key primary fingerprint does not match pin")
        status = gpg(home, "--status-fd", "1", "--verify",
                     str(manifest) + ".asc", str(manifest))
        require_validsig(status, expected)
    subprocess.run(["sha256sum", "--check", manifest.name],
                   cwd=manifest.parent, check=True)


def verify(manifest):
    _verify(manifest, PUBLIC_KEY.read_bytes(), PRIMARY_FINGERPRINT)


def _sign(manifest, key, passphrase, subkey, public_key, expected):
    with gpg_home() as home:
        gpg(home, "--import", data=key)
        gpg(home, "--pinentry-mode", "loopback", "--passphrase-fd", "0",
            "--local-user", subkey + "!", "--armor", "--detach-sign",
            "--output", str(manifest) + ".asc", str(manifest),
            data=passphrase + b"\n")
        # A separate empty home imports only the supplied public key.
        _verify(manifest, public_key, expected)


def sign(manifest):
    _sign(manifest, os.environ.pop("RELEASE_SIGNING_KEY").encode(),
          os.environ.pop("RELEASE_SIGNING_PASSPHRASE").encode(), SIGNING_SUBKEY,
          PUBLIC_KEY.read_bytes(), PRIMARY_FINGERPRINT)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=["sign", "verify"])
    parser.add_argument("manifest", type=lambda p: Path(p).resolve())
    args = parser.parse_args()
    try:
        {"sign": sign, "verify": verify}[args.operation](args.manifest)
    except (ValueError, subprocess.CalledProcessError, KeyError):
        # Never print command inputs or secret-bearing process exceptions.
        raise SystemExit("Release signature/checksum operation failed") from None
