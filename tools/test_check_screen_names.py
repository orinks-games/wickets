import tempfile
import unittest
from pathlib import Path

import check_screen_names as csn


def write(root: Path, rel: str, text: str) -> None:
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


STATE = """
impl State<Parlor> for {typ} {{
    fn name(&self) -> &'static str {{
        {expr}
    }}
}}
"""


class ScreenNames(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)

    def tearDown(self):
        self.dir.cleanup()

    def names(self):
        return sorted(n for n, *_ in csn.screens(self.root) if n is not None)

    def test_literal_const_and_macro_names_are_read(self):
        write(
            self.root,
            "games/tea-room/src/lib.rs",
            'const PLAY: &str = "tea_room_play";\n'
            + STATE.format(typ="Play", expr="PLAY")
            + STATE.format(typ="Shelf", expr='"tea_room_shelf"')
            + "macro_rules! menu_impl { ($t:ty, $name:expr) => {\n"
            "    fn name(&self) -> &'static str { $name }\n} }\n"
            'impl Menu for Shop { menu_impl!(Shop, "tea_room_shop"); }\n',
        )
        self.assertEqual(self.names(), ["tea_room_play", "tea_room_shelf", "tea_room_shop"])
        self.assertEqual(csn.problems(csn.screens(self.root)), [])

    def test_a_bare_name_in_a_game_is_refused(self):
        write(self.root, "games/tea-room/src/lib.rs", STATE.format(typ="Play", expr='"kitchen"'))
        found = csn.problems(csn.screens(self.root))
        self.assertEqual(len(found), 1)
        self.assertIn("'kitchen'", found[0])
        self.assertIn("'tea_room_'", found[0])

    def test_a_game_may_wrap_a_shared_screen(self):
        write(self.root, "crates/core/src/pages.rs", STATE.format(typ="Pages", expr='"pages"'))
        write(self.root, "games/tea-room/src/run.rs", STATE.format(typ="PlayPage", expr='"pages"'))
        self.assertEqual(csn.problems(csn.screens(self.root)), [])

    def test_one_name_on_two_screens_is_refused(self):
        write(self.root, "games/tea-room/src/a.rs", STATE.format(typ="A", expr='"tea_room_play"'))
        write(self.root, "games/tea-room/src/b.rs", STATE.format(typ="B", expr='"tea_room_play"'))
        found = csn.problems(csn.screens(self.root))
        self.assertEqual(len(found), 1)
        self.assertIn("more than one screen", found[0])

    def test_the_overlay_copy_of_a_screen_is_the_same_screen(self):
        write(self.root, "games/tea-room/src/public.rs", STATE.format(typ="A", expr='"tea_room_play"'))
        write(self.root, "secrets/games/tea-room/src/lib.rs", STATE.format(typ="A", expr='"tea_room_play"'))
        write(self.root, "crates/core/src/menu.rs", STATE.format(typ="Base", expr='"base"'))
        write(self.root, "secrets/crates/core/src/menu.rs", STATE.format(typ="Base", expr='"base"'))
        self.assertEqual(csn.problems(csn.screens(self.root)), [])

    def test_test_modules_and_comments_are_skipped(self):
        write(
            self.root,
            "games/tea-room/src/lib.rs",
            STATE.format(typ="Play", expr='"tea_room_play"')
            + "// impl State<Parlor> for Old { fn name(&self) -> &'static str { \"old\" } }\n"
            + "#[cfg(test)]\nmod tests {\n" + STATE.format(typ="Dummy", expr='"run"') + "}\n",
        )
        self.assertEqual(self.names(), ["tea_room_play"])


if __name__ == "__main__":
    unittest.main()
