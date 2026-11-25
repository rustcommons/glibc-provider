"""Release writes are modeled locally; mismatches must never replace assets."""

import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import publish_release_assets as release


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.dist = Path(temp.name)
        self.version = '9.9.9'
        names = ['glibc-provider-sources-9.9.9.crate',
                 'glibc-provider-sources-9.9.9.cdx.json', 'SHA256SUMS',
                 'SHA256SUMS.asc', 'glibc-provider-9.9.9.crates.intoto.jsonl']
        self.files = {name: name.encode() for name in names}
        for name, data in self.files.items():
            (self.dist / name).write_bytes(data)
        self.release = dict(id=42, tag_name='v9.9.9', name='Release 9.9.9',
                            body='# Release 9.9.9\n', prerelease=False,
                            draft=True, assets=[])
        self.downloads = {}
        self.writes = []
        self.exists = True

    def add_asset(self, name):
        asset_id = len(self.release['assets']) + 100
        data = self.files[name]
        self.release['assets'].append(dict(id=asset_id, name=name, size=len(data)))
        self.downloads[asset_id] = data

    def gh(self, *args):
        if args[:3] == ('api', '--paginate', '--slurp'):
            return json.dumps([[self.release] if self.exists else []]).encode()
        if args == ('api', 'repos/example/repo/releases/42'):
            return json.dumps(self.release).encode()
        if args[:2] == ('release', 'create'):
            self.assertIn('--draft', args)
            self.assertIn('--verify-tag', args)
            self.exists = True
            self.writes.append(args)
            return b''
        if args[:2] == ('release', 'upload'):
            self.writes.append(args)
            self.assertTrue(self.release['draft'])
            self.assertNotIn('--clobber', args)
            self.add_asset(Path(args[3]).name)
            return b''
        if args[:3] == ('api', '--method', 'PATCH'):
            self.writes.append(args)
            self.assertEqual(len(self.release['assets']), len(self.files))
            self.release['draft'] = False
            return b'{}'
        if args[0] == 'api' and '/releases/assets/' in args[1]:
            return self.downloads[int(args[1].rsplit('/', 1)[1])]
        raise AssertionError(args)

    def publish(self):
        original = Path.read_text
        def read(path, *args, **kwargs):
            if str(path) == 'docs/releases/v9.9.9.md':
                return '# Release 9.9.9\n'
            return original(path, *args, **kwargs)
        with patch.object(release, 'gh', side_effect=self.gh), patch.object(Path, 'read_text', read), patch.object(release, 'verify'):
            release.publish('example/repo', self.version, self.dist)

    def test_create_draft_before_uploads(self):
        self.exists = False
        self.publish()
        self.assertEqual(self.writes[0][:2], ('release', 'create'))
        self.assertFalse(self.release['draft'])

    def test_resume_draft_and_publish_signature(self):
        self.add_asset('SHA256SUMS')
        self.publish()
        self.assertFalse(self.release['draft'])
        self.assertEqual(len(self.writes), 5)  # four uploads and publish
        self.assertIn('SHA256SUMS.asc', [a['name'] for a in self.release['assets']])

    def test_published_release_is_read_only(self):
        for name in self.files:
            self.add_asset(name)
        self.release['draft'] = False
        self.publish()
        self.assertEqual(self.writes, [])

    def test_missing_signature_on_published_release_is_refused(self):
        for name in self.files:
            if name != 'SHA256SUMS.asc':
                self.add_asset(name)
        self.release['draft'] = False
        with self.assertRaisesRegex(ValueError, 'Missing published asset'):
            self.publish()
        self.assertEqual(self.writes, [])

    def test_size_and_same_size_digest_mismatch_refused(self):
        self.add_asset('SHA256SUMS')
        asset = self.release['assets'][0]
        for data in (b'wrong size', b'x' * asset['size']):
            with self.subTest(data=data):
                self.downloads[asset['id']] = data
                # Test size field and bytes independently.
                asset['size'] = len(data)
                with self.assertRaisesRegex(ValueError, 'mismatch'):
                    self.publish()
                self.assertEqual(self.writes, [])

    def test_invalid_existing_signature_refused(self):
        self.add_asset('SHA256SUMS.asc')
        with patch.object(release, 'gh', side_effect=self.gh), patch.object(
                release, 'verify', side_effect=ValueError('invalid signature')):
            with self.assertRaisesRegex(ValueError, 'invalid signature'):
                release.check_asset(self.release['assets'][0],
                                    self.dist / 'SHA256SUMS.asc', 'example/repo')
        self.assertEqual(self.writes, [])

    def test_rerun_keeps_valid_signature_with_different_timestamp(self):
        self.add_asset('SHA256SUMS.asc')
        asset = self.release['assets'][0]
        self.downloads[asset['id']] = b'previous valid signature'
        asset['size'] = len(self.downloads[asset['id']])
        self.publish()
        uploads = [Path(args[3]).name for args in self.writes if args[:2] == ('release', 'upload')]
        self.assertNotIn('SHA256SUMS.asc', uploads)

    def test_metadata_and_unexpected_assets_refused(self):
        self.release['body'] = 'different notes'
        with self.assertRaisesRegex(ValueError, 'metadata'):
            self.publish()
        self.assertEqual(self.writes, [])


if __name__ == '__main__':
    unittest.main()
