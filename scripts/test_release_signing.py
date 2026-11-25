"""Real GnuPG round trips use only an ephemeral, passphrase-protected subkey."""

import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import release_signing as signing


class SigningTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.homes = signing.gpg_home()
        cls.home = cls.homes.__enter__()
        cls.addClassCleanup(cls.homes.__exit__, None, None, None)
        cls.password = b"throwaway-test-passphrase"
        signing.gpg(cls.home, "--pinentry-mode", "loopback", "--passphrase-fd", "0",
                    "--quick-generate-key", "Release test <test@example.invalid>",
                    "ed25519", "cert", "1d", data=cls.password + b"\n")
        cls.primary = signing.primary_fingerprint(cls.home)
        signing.gpg(cls.home, "--pinentry-mode", "loopback", "--passphrase-fd", "0",
                    "--quick-add-key", cls.primary, "ed25519", "sign", "1d",
                    data=cls.password + b"\n")
        records = signing.gpg(cls.home, "--with-colons", "--list-keys").decode()
        cls.subkey = [r.split(":")[9] for r in records.splitlines()
                      if r.startswith("fpr:")][-1]
        cls.public = signing.gpg(cls.home, "--armor", "--export")
        cls.secret = signing.gpg(cls.home, "--pinentry-mode", "loopback",
                                "--passphrase-fd", "0", "--armor", "--export-secret-subkeys",
                                data=cls.password + b"\n")

    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.asset = self.root / "candidate.crate"
        self.asset.write_bytes(b"candidate bytes")
        self.manifest = self.root / "SHA256SUMS"
        self.manifest.write_text(hashlib.sha256(self.asset.read_bytes()).hexdigest()
                                 + "  candidate.crate\n")

    def sign_test(self):
        signing._sign(self.manifest, self.secret, self.password, self.subkey,
                      self.public, self.primary)

    def test_committed_key_has_pinned_primary(self):
        with signing.gpg_home() as home:
            signing.gpg(home, "--import", data=signing.PUBLIC_KEY.read_bytes())
            self.assertEqual(signing.primary_fingerprint(home), signing.PRIMARY_FINGERPRINT)

    def test_subkey_round_trip_and_cleanup(self):
        self.assertNotEqual(self.primary, self.subkey)
        self.sign_test()
        signing._verify(self.manifest, self.public, self.primary)
        with signing.gpg_home() as home:
            self.assertEqual(os.stat(home).st_mode & 0o777, 0o700)
        self.assertFalse(Path(home).exists())

    def test_tampered_manifest(self):
        self.sign_test()
        self.manifest.write_text(self.manifest.read_text() + "tampered\n")
        with self.assertRaises(subprocess.CalledProcessError):
            signing._verify(self.manifest, self.public, self.primary)

    def test_tampered_or_missing_asset(self):
        self.sign_test()
        self.asset.write_bytes(b"tampered")
        with self.assertRaises(subprocess.CalledProcessError):
            signing._verify(self.manifest, self.public, self.primary)
        self.asset.unlink()
        with self.assertRaises(subprocess.CalledProcessError):
            signing._verify(self.manifest, self.public, self.primary)

    def test_wrong_passphrase(self):
        # Independent imported home has no agent cache of the password.
        with self.assertRaises(subprocess.CalledProcessError):
            signing._sign(self.manifest, self.secret, b"wrong", self.subkey,
                          self.public, self.primary)

    def test_production_verifier_rejects_throwaway_key(self):
        self.sign_test()
        with patch.object(signing, 'PUBLIC_KEY', self.root / 'test.asc'):
            signing.PUBLIC_KEY.write_bytes(self.public)
            with self.assertRaisesRegex(ValueError, "fingerprint"):
                signing.verify(self.manifest)

    def test_cli_has_no_test_fingerprint_option(self):
        result = subprocess.run(["python3", str(Path(signing.__file__)), "verify",
                                 str(self.manifest), "--fingerprint", self.primary],
                                capture_output=True)
        self.assertEqual(result.returncode, 2)

    def test_status_parser_requires_primary_not_subkey(self):
        status = (f"[GNUPG:] VALIDSIG {self.subkey} 2026-11-25 1 0 4 0 22 8 00 "
                  f"{self.primary}\n").encode()
        signing.require_validsig(status, self.primary)
        for invalid in (b"", b"[GNUPG:] GOODSIG key name\n", status + status):
            with self.assertRaises(ValueError):
                signing.require_validsig(invalid, self.primary)
        with self.assertRaises(ValueError):
            signing.require_validsig(status, self.subkey)


if __name__ == '__main__':
    unittest.main()
