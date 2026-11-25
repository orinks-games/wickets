# Translating Wicket's

Everything Wicket's speaks can be translated: menus, Settings, every game,
tutorials, help, results and story. Sounds and music stay as they are. The
holiday characters' recorded voices are in English: in another language the
game plays as it does with their voice switched off.

The first translations were written by Claude, an AI, so they will have
mistakes a fluent player catches in seconds. Corrections are welcome from
anyone who plays in one of these languages. You do not need to know how to
program.

| File | Language |
| --- | --- |
| `es.po` | Spanish |
| `fr.po` | French |
| `de.po` | German |
| `pt_BR.po` | Brazilian Portuguese |
| `it.po` | Italian |
| `ru.po` | Russian |
| `pl.po` | Polish |
| `zh_CN.po` | Simplified Chinese |
| `ja.po` | Japanese |
| `tr.po` | Turkish |
| `vi.po` | Vietnamese |

To play in one of them, open Settings, then Speech; Language is the first
row. Left and Right change it. Automatic follows your system language. A
screen reader speaks the game in your language; the built-in voice speaks
English only.

## Report a wrong line

Open an issue at https://github.com/orinks-games/wickets/issues with the
language, what the game said, and what it should say. Your screen reader's
speech history has the exact words. A maintainer puts the fix into the file
for you.

## Fix it yourself

Every release has a translation kit beside the game's downloads, on
https://orinks.net/wickets/downloads and on the release's GitHub page:
`wickets-<version>-translation-kit.zip`. It holds this folder's language
files, the template, the glossaries and this guide. The files are also in
this folder on GitHub, where they can be newer than the last release.

Each language is one text file, in the gettext PO format.
A translation editor such as [Poedit](https://poedit.net) opens it, and so
does any plain text editor. Each entry is the English line, then its
translation:

```po
msgid "{score} points, your best."
msgstr "{score} puntos, tu mejor marca."
```

Change the `msgstr` line and leave the `msgid` line exactly as it is; the
game finds a line by its English. Then open a pull request, or attach the
file to an issue.

### Hear your change before sending it

Put your edited file in a folder named `lang` next to the game's program,
then restart the game. The lines your file translates replace the built-in
ones; every line it leaves empty, marks fuzzy or does not have keeps the
built-in translation, so a file from an older release or one that covers
only part of the game works too. Delete it to go back to the built-in
translation. The saves folder's
`lang` folder works too; when both have a file for the same language, the
one next to the program wins.

To list every line the game said in English because no translation matched,
set the environment variable `ECHO_PARLOR_L10N_MISSES` to a file path before
starting the game. `ECHO_PARLOR_LANG=es` starts the game in one language for
one run.

## Start a new language

`wickets.pot` in this folder is the template: every line the game can say,
untranslated. Copy it to `<code>.po` (such as `nl.po` for Dutch), set
`Language`, `X-Language-Name` and `X-Language-Autonym` in its header, and
translate as much as you like; every empty entry is said in English.

## Rules for every line

1. **Placeholders stay whole.** A word in braces, like `{name}` or
   `{score}`, is filled in by the game. Move it wherever your grammar
   needs it, but copy the name inside the braces exactly. A translation that
   names a placeholder the English line does not have is ignored.
2. **Numbers stay as written.** Every digit in the English line appears in
   the translation.
3. **Key names stay in English**: Enter, Escape, Space, Tab, Page Up, F1 and
   the rest. They are the names on the keyboard and in the game's key
   settings.
4. **Names stay in English**: Wicket's, every game's name, and every
   person's name.
5. **One word for each thing.** Use the term in your language's glossary,
   `glossary/<code>.md`, every time. The English terms and what they mean
   are in [glossary/en.md](glossary/en.md).
6. **Write for the ear, and short.** A screen reader will say every word.
   Short, plain sentences; no symbols that are read out as their names; no
   parentheses. Keep each line about as long as the English; the checker
   flags one far longer.
7. **No song titles.** No line names the tune a piece of music is based on.
8. **Leave an entry empty** if you are unsure: an empty `msgstr` makes the
   game say that line in English. Mark a guess you want checked with a
   `#, fuzzy` line above it; the game skips fuzzy entries too.

### Plurals

English picks "1 lamp" or "2 lamps" in code, so both forms usually appear
as separate lines. When your language needs a word chosen by a number, write
the forms after the placeholder's name, separated by `|`:

```po
msgid "{count} lamps lit."
msgstr "Горит {count} {count:лампа|лампы|ламп}."
```

The forms are in this order:

- Spanish, Portuguese, Italian, German, Turkish: singular, plural.
- French: singular (0 and 1), plural.
- Russian: one (1, 21, 31), few (2 to 4, 22 to 24), many (5 to 20, 25).
- Polish: one (1 only), few (2 to 4, 22 to 24), many (the rest).
- Chinese, Japanese: no plurals; one form, or none at all.

`{s_ending}` is the English plural "s" and nothing else. Leave it out, and
choose the word by the number instead:

```po
msgid "{n} round{s_ending} left"
msgstr "{n} {n:ronda|rondas} más"
```

### Lines built from pieces

Some lines are assembled from smaller pieces at run time, so the whole
sentence is not in the file. The game translates each sentence it can
match whole. A sentence it cannot match whole is split into clauses only if
every clause (other than a name or a number) is in the file; otherwise the
sentence stays in English, so a player never hears one sentence in two
languages. Entries marked "one clause of a longer line" are those clauses.
If you hear a line that stays English, report it in an issue
and we will add it.

A menu row's position ("4 of 8.") is its own entry, `{position} of
{count}.`, spoken after whichever row has focus.

### Two entries with the same English

An entry with a `msgctxt` line is a second use of the same English: the
`controller button` "Back" is the name of the controller's Back button,
which stays as the controller prints it. The plain "Back" is the menu row
that leaves a screen; translate that one.

## For maintainers

- `lang/wickets.pot` is the template: every line the game can say, with the
  file it comes from. It is generated; do not edit it by hand.
  `lang/sources.txt` lists where the lines come from, including Hearken's
  own menus.
- `lang/extra.pot` holds lines the source never spells out whole, added by
  hand. Each one is a template like the others.
- After changing any player text, run `cargo run -p wickets-l10n -- extract`.
  It rewrites the template and adds new lines to every language file,
  untranslated. CI fails while the files are out of date (`extract
  --check`).
- `cargo run -p wickets-l10n -- check` checks every language file:
  placeholders, numbers, key names, plural forms, length and symbols.
  `--stats` prints how much of each language is translated.
- Lines that only the private overlay has live in its own `lang/` folder,
  with its own template. The full build merges the two for each language.
- Before a tester build, translate the new lines in every language.
