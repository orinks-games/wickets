"""Tests for check_channel_manifests.py: python3 -m unittest discover -s tools."""

import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import check_channel_manifests as check  # noqa: E402


def write(folder: Path, channel: str, signed_channel=None, built=None, platform="windows-x64"):
    manifest = {"format": 1, "channel": signed_channel or channel, "platform": platform}
    if built:
        manifest["built"] = built
    envelope = {"format": 1, "signature": "00", "signed": json.dumps(manifest)}
    (folder / f"update-{channel}-{platform}.json").write_text(json.dumps(envelope))


class ReleaseFiles(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def test_each_channel_with_only_its_own_manifest_passes(self):
        for channel in ["stable", "beta", "alpha"]:
            with self.subTest(channel=channel):
                for f in self.dir.glob("*"):
                    f.unlink()
                write(self.dir, channel, built=channel)
                self.assertEqual(check.check_dir(self.dir, channel), [])

    def test_an_alpha_manifest_on_a_beta_release_fails(self):
        # What 1.4.0-beta.3 published.
        write(self.dir, "beta")
        write(self.dir, "alpha")
        self.assertEqual(len(check.check_dir(self.dir, "beta")), 1)

    def test_any_other_channel_on_a_stable_release_fails(self):
        # What 1.3.2 published.
        write(self.dir, "stable")
        write(self.dir, "beta")
        write(self.dir, "alpha")
        self.assertEqual(len(check.check_dir(self.dir, "stable")), 2)

    def test_a_manifest_signed_for_another_channel_than_its_name_fails(self):
        write(self.dir, "alpha", signed_channel="beta")
        self.assertEqual(len(check.check_dir(self.dir, "alpha")), 1)

    def test_a_package_built_for_another_channel_fails(self):
        write(self.dir, "alpha", built="beta")
        self.assertEqual(len(check.check_dir(self.dir, "alpha")), 1)

    def test_no_manifest_fails(self):
        self.assertEqual(len(check.check_dir(self.dir, "beta")), 1)


class ReleaseWorkflow(unittest.TestCase):
    def test_the_release_workflow_publishes_only_its_own_channel(self):
        self.assertEqual(check.check_workflow(check.WORKFLOW.read_text(encoding="utf-8")), [])

    def test_signing_again_for_other_channels_fails(self):
        text = """
          hearken release --channel="$CHANNEL"
          cargo run -p channel-manifest -- "$MANIFEST" beta alpha
          gh release upload "$GITHUB_REF_NAME" "$FILE" --clobber
          gh release upload "$GITHUB_REF_NAME" "$m" --clobber
          gh release upload "$GITHUB_REF_NAME" "$MANIFEST" --clobber
        """
        self.assertEqual(len(check.check_workflow(text)), 2)

    def test_packaging_for_a_fixed_channel_fails(self):
        text = """
          hearken release --channel=alpha
          gh release upload "$GITHUB_REF_NAME" "$MANIFEST" --clobber
          gh release upload "$GITHUB_REF_NAME" dist/update-alpha-linux-x64.json
        """
        self.assertEqual(len(check.check_workflow(text)), 3)


if __name__ == "__main__":
    unittest.main()
