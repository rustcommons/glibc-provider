"""Publication checksum checks need no registry writes or credentials."""

import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import urllib.error

spec = importlib.util.spec_from_file_location(
    "verifier", Path(__file__).with_name("verify-crate-checksum.py")
)
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class ChecksumTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.archive = Path(self.temp.name) / "candidate.crate"
        self.data = b"candidate archive"
        self.archive.write_bytes(self.data)
        self.digest = hashlib.sha256(self.data).hexdigest()

    def metadata(self, checksum=None, yanked=False):
        return io.BytesIO(json.dumps({"version": {
            "checksum": checksum or self.digest, "yanked": yanked,
        }}).encode())

    def verify(self, responses, attempts=1):
        with patch.object(verifier.urllib.request, "urlopen", side_effect=responses):
            verifier.verify({"name": "example", "version": "1.0.0"},
                            self.archive, attempts=attempts, delay=0)

    def test_matching_metadata_and_archive(self):
        self.verify([self.metadata(), io.BytesIO(self.data)])

    def test_metadata_mismatch(self):
        with self.assertRaisesRegex(SystemExit, "Registry checksum mismatch"):
            self.verify([self.metadata("0" * 64)])

    def test_yanked(self):
        with self.assertRaisesRegex(SystemExit, "yanked"):
            self.verify([self.metadata(yanked=True)])

    def test_archive_mismatch(self):
        with self.assertRaisesRegex(SystemExit, "Official archive checksum mismatch"):
            self.verify([self.metadata(), io.BytesIO(b"different archive")])

    def test_visibility_retry(self):
        unavailable = urllib.error.HTTPError("url", 404, "not visible", {}, None)
        self.verify([unavailable, self.metadata(), io.BytesIO(self.data)], attempts=2)

    def test_visibility_exhaustion(self):
        unavailable = urllib.error.HTTPError("url", 404, "not visible", {}, None)
        with self.assertRaises(urllib.error.HTTPError):
            self.verify([unavailable])


if __name__ == "__main__":
    unittest.main()
