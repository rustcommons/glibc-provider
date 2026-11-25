"""Real GnuPG round trips use only an ephemeral, passphrase-protected subkey."""

import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
import time
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
        with self.assertRaises(ValueError):
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

    def historic_key(self, home, created):
        signing.gpg(home, "--faked-system-time", str(created), "--pinentry-mode",
                    "loopback", "--passphrase", "", "--quick-generate-key",
                    "Historic test <historic@example.invalid>", "ed25519", "cert", "0")
        primary = signing.primary_fingerprint(home)
        signing.gpg(home, "--faked-system-time", str(created), "--pinentry-mode",
                    "loopback", "--passphrase", "", "--quick-add-key",
                    primary, "ed25519", "sign", "1d")
        listing = signing.gpg(home, "--with-colons", "--list-keys").decode()
        subkey = [line.split(":")[9] for line in listing.splitlines()
                  if line.startswith("fpr:")][-1]
        signing.gpg(home, "--faked-system-time", str(created + 60),
                    "--pinentry-mode", "loopback", "--passphrase", "",
                    "--local-user", subkey + "!", "--armor", "--detach-sign",
                    "--output", str(self.manifest) + ".asc", str(self.manifest))
        return primary

    def test_expired_subkey_signed_before_expiry_is_accepted(self):
        with signing.gpg_home() as home:
            primary = self.historic_key(home, int(time.time()) - 3 * 86400)
            status = signing.gpg(home, "--status-fd", "1", "--verify",
                                 str(self.manifest) + ".asc", str(self.manifest), check=False)
            self.assertIn(b"[GNUPG:] EXPKEYSIG ", status)
            public = signing.gpg(home, "--armor", "--export")
        signing._verify(self.manifest, public, primary)

    def test_revoked_subkey_is_refused(self):
        with signing.gpg_home() as home:
            primary = self.historic_key(home, int(time.time()) - 600)
            signing.gpg(home, "--command-fd", "0", "--yes", "--edit-key", primary,
                        data=b"key 1\nrevkey\ny\n0\n\ny\nsave\n")
            status = signing.gpg(home, "--status-fd", "1", "--verify",
                                 str(self.manifest) + ".asc", str(self.manifest), check=False)
            self.assertIn(b"[GNUPG:] REVKEYSIG ", status)
            self.assertIn(b"[GNUPG:] VALIDSIG ", status)
            public = signing.gpg(home, "--armor", "--export")
        with self.assertRaisesRegex(ValueError, "revoked"):
            signing._verify(self.manifest, public, primary)

    def test_status_parser_requires_primary_not_subkey(self):
        status = (f"[GNUPG:] GOODSIG {self.subkey[-16:]} test\n"
                  f"[GNUPG:] VALIDSIG {self.subkey} 2026-11-25 1 0 4 0 22 8 00 "
                  f"{self.primary}\n").encode()
        signing.require_validsig(status, self.primary)
        for invalid in (b"", b"[GNUPG:] GOODSIG key name\n", status + status):
            with self.assertRaises(ValueError):
                signing.require_validsig(invalid, self.primary)
        with self.assertRaises(ValueError):
            signing.require_validsig(status, self.subkey)

    def test_status_rejections_and_expiry_boundary(self):
        valid = (f"[GNUPG:] GOODSIG key test\n[GNUPG:] VALIDSIG {self.subkey} "
                 f"2026-11-25 100 0 4 0 22 8 00 {self.primary}\n").encode()
        for refused in ("REVKEYSIG", "KEYREVOKED", "BADSIG", "ERRSIG", "EXPSIG", "NO_PUBKEY"):
            with self.subTest(status=refused), self.assertRaises(ValueError):
                signing.require_validsig(valid + f"[GNUPG:] {refused} key\n".encode(), self.primary)
        for invalid in (valid.replace(b"GOODSIG", b"UNKNOWN"),
                        valid + b"[GNUPG:] GOODSIG key test\n"):
            with self.assertRaises(ValueError):
                signing.require_validsig(invalid, self.primary)
        expired = valid.replace(b"GOODSIG", b"EXPKEYSIG") + b"[GNUPG:] KEYEXPIRED 100\n"
        for expiry in ("", "0", "99", "100", "unknown"):
            listing = f"sub::::::{expiry}:\nfpr:::::::::{self.subkey}:\n".encode()
            with self.subTest(expiry=expiry), self.assertRaises(ValueError):
                signing.require_validsig(expired, self.primary, listing)
        listing = f"sub::::::101:\nfpr:::::::::{self.subkey}:\n".encode()
        signing.require_validsig(expired, self.primary, listing)
        signing.require_validsig(valid + b"[GNUPG:] KEYEXPIRED 101\n", self.primary, listing)


if __name__ == '__main__':
    unittest.main()
