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


def gpg(home, *args, data=None, check=True):
    # Do not pass signing secrets through the child environment or diagnostics.
    env = {k: v for k, v in os.environ.items()
           if k not in {"RELEASE_SIGNING_KEY", "RELEASE_SIGNING_PASSPHRASE"}}
    return subprocess.run(["gpg", "--homedir", home, "--batch", *args], input=data,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                          check=check, env=env).stdout


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


def require_validsig(status, expected, key_listing=b""):
    records = [line.split()[1:] for line in status.decode().splitlines()
               if line.startswith("[GNUPG:] ")]
    refused = {"REVKEYSIG", "KEYREVOKED", "BADSIG", "ERRSIG", "EXPSIG", "NO_PUBKEY"}
    if any(record[0] in refused for record in records):
        raise ValueError("Signature has a revoked, bad, expired or unknown-key status")
    good = [record for record in records if record[0] in {"GOODSIG", "EXPKEYSIG"}]
    signatures = [record[1:] for record in records if record[0] == "VALIDSIG"]
    # VALIDSIG: signing fingerprint, date, timestamp, expiry, version,
    # reserved, pubkey algo, hash algo, signature class, primary fingerprint.
    if len(good) != 1 or len(signatures) != 1 or len(signatures[0]) != 10:
        raise ValueError("Expected exactly one good signature and complete VALIDSIG record")
    signature = signatures[0]
    if signature[-1] != expected:
        raise ValueError("Signature primary fingerprint does not match pin")
    # KEYEXPIRED can refer to an unrelated sibling subkey.
    if any(record[0] == "EXPKEYSIG" for record in records):
        expiries = {}
        expiry = ""
        for line in key_listing.decode().splitlines():
            fields = line.split(":")
            if fields[0] in {"pub", "sub"}:
                expiry = fields[6]
            elif fields[0] == "fpr":
                expiries[fields[9]] = expiry
        expiry = expiries.get(signature[0], "")
        if (not expiry.isdecimal() or not signature[2].isdecimal()
                or int(signature[2]) >= int(expiry)):
            raise ValueError("Signature was not made before signing key expiry")


def _verify(manifest, public_key, expected):
    with gpg_home() as home:
        gpg(home, "--import", data=public_key)
        if primary_fingerprint(home) != expected:
            raise ValueError("Public key primary fingerprint does not match pin")
        status = gpg(home, "--status-fd", "1", "--verify",
                     str(manifest) + ".asc", str(manifest), check=False)
        require_validsig(status, expected, gpg(home, "--with-colons", "--list-keys"))
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
