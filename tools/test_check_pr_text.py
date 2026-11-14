import io
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

import check_pr_text as cpt


class PrText(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)
        self.private = self.root / "secrets"
        self.private.mkdir()
        (self.private / "spoiler-words.txt").write_text(
            "# comment\nzzsecret\nZZOther Thing\nzzshared\n", encoding="utf-8"
        )
        (self.private / "spoiler-allow-pr.txt").write_text(
            "# fine in PR text\nzzshared\n", encoding="utf-8"
        )

    def tearDown(self):
        self.dir.cleanup()

    def run_main(self, args, present=True, stdin=""):
        words = self.private / "spoiler-words.txt"
        if not present:
            words = self.private / "missing.txt"
        out = io.StringIO()
        with mock.patch.object(cpt, "WORDS", words), mock.patch.object(
            cpt, "ALLOW_PR", self.private / "spoiler-allow-pr.txt"
        ), mock.patch("sys.stdin", io.StringIO(stdin)), redirect_stdout(out):
            code = cpt.main(args)
        return code, out.getvalue()

    def file(self, name, text):
        path = self.root / name
        path.write_text(text, encoding="utf-8")
        return str(path)

    def test_allowed_terms_are_dropped(self):
        terms = cpt.load_terms(
            self.private / "spoiler-words.txt",
            self.private / "spoiler-allow-pr.txt",
        )
        self.assertEqual(terms, ["zzsecret", "zzother thing"])

    def test_missing_allow_file_drops_nothing(self):
        terms = cpt.load_terms(
            self.private / "spoiler-words.txt", self.private / "nope.txt"
        )
        self.assertIn("zzshared", terms)

    def test_clean_text_passes(self):
        path = self.file("title", "Add a lake game\n")
        code, out = self.run_main([path])
        self.assertEqual((code, out), (0, ""))

    def test_allowed_term_passes(self):
        path = self.file("description", "mentions zzshared here\n")
        self.assertEqual(self.run_main([path])[0], 0)

    def test_hit_names_input_and_line_but_not_term(self):
        path = self.file("description", "fine\nsecond ZzSecret line\n")
        code, out = self.run_main([path])
        self.assertEqual(code, 1)
        self.assertEqual(out, "description:2\n")
        self.assertNotIn("zzsecret", out.casefold())

    def test_multi_word_term_matches(self):
        path = self.file("commit-abc1234", "a zzother thing b\n")
        code, out = self.run_main([path])
        self.assertEqual((code, out), (1, "commit-abc1234:1\n"))

    def test_every_failing_input_is_listed(self):
        a = self.file("title", "zzsecret\n")
        b = self.file("commit-1111111", "ok\n")
        c = self.file("commit-2222222", "ok\nzzsecret\n")
        code, out = self.run_main([a, b, c])
        self.assertEqual(code, 1)
        self.assertEqual(out, "title:1\ncommit-2222222:2\n")

    def test_stdin(self):
        code, out = self.run_main([], stdin="x\nzzsecret\n")
        self.assertEqual((code, out), (1, "stdin:2\n"))
        code, out = self.run_main(["-"], stdin="clean\n")
        self.assertEqual((code, out), (0, ""))

    def test_missing_overlay(self):
        path = self.file("title", "zzsecret\n")
        code, out = self.run_main([path], present=False)
        self.assertEqual(code, 0)
        self.assertIn("not present", out)
        code, out = self.run_main(["--require", path], present=False)
        self.assertEqual(code, 1)
        self.assertIn("not present", out)


if __name__ == "__main__":
    unittest.main()
