"""Tests for release_channels.py: python3 -m unittest discover -s tools."""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

import release_channels as rc  # noqa: E402


def release(tag, *channels, draft=False):
    assets = [{"name": f"update-{c}-windows-x64.json"} for c in channels]
    assets.append({"name": f"wickets-{tag[1:]}-windows-x64.zip"})
    return {"tag_name": tag, "draft": draft, "assets": assets}


class Plan(unittest.TestCase):
    def test_each_channel_reaches_the_less_steady_ones(self):
        self.assertEqual(rc.plan("v1.9.0", "stable", [])[0], ["stable", "beta", "alpha"])
        self.assertEqual(rc.plan("v1.9.0-beta.1", "beta", [])[0], ["beta", "alpha"])
        self.assertEqual(rc.plan("v1.9.0-alpha.1", "alpha", [])[0], ["alpha"])

    def test_a_stable_after_its_alphas_reaches_alpha(self):
        releases = [release("v1.9.0-alpha.2", "alpha"), release("v1.8.0", "stable")]
        self.assertEqual(rc.plan("v1.9.0", "stable", releases)[0], ["stable", "beta", "alpha"])

    def test_a_patch_leaves_out_a_channel_with_a_newer_release(self):
        releases = [
            release("v1.9.0-alpha.2", "alpha"),
            release("v1.8.2", "stable", "beta", "alpha"),
        ]
        build, skipped = rc.plan("v1.8.3", "stable", releases)
        self.assertEqual(build, ["stable", "beta"])
        self.assertIn("v1.9.0-alpha.2", skipped[0])

    def test_an_equal_version_counts_as_newer(self):
        releases = [release("v1.9.0+rebuilt", "alpha")]
        self.assertEqual(rc.plan("v1.9.0", "stable", releases)[0], ["stable", "beta"])

    def test_its_own_release_and_drafts_do_not_count(self):
        releases = [
            release("v1.9.0", "stable", "alpha"),
            release("v2.0.0-alpha.1", "alpha", draft=True),
        ]
        self.assertEqual(rc.plan("v1.9.0", "stable", releases)[0], ["stable", "beta", "alpha"])

    def test_prereleases_sort_by_semver(self):
        key = rc.version_key
        self.assertLess(key("v1.9.0-alpha.2"), key("v1.9.0-alpha.10"))
        self.assertLess(key("v1.9.0-alpha.9"), key("v1.9.0-beta.1"))
        self.assertLess(key("v1.9.0-beta.1"), key("v1.9.0"))
        self.assertLess(key("v1.9.0"), key("v1.10.0-alpha.1"))
        self.assertIsNone(key("nightly"))


if __name__ == "__main__":
    unittest.main()
