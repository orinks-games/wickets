# Wicket's story files

Everything story mode says lives here, as plain text the game builds in.
Writing a scene means editing one of these files. No Rust is needed.

| File | What it holds |
| --- | --- |
| `main.md` | The main plot, told once, as lamps light. |
| `regulars/<who>.md` | One regular's side story, five scenes. |
| `doors.md` | A short scene for each game added after the first 27. |
| `visits/<holiday>.md` | A holiday's visitor, with scenes for each year. |
| `chapters/NN-name.md` | The story after The keys, one chapter a file. |
| `talk.md` | The host's opening line for a sitting. |
| `shipped.txt` | Every scene id that has shipped. |

## The format

```
> A note for writers. Never read out.

# The new keeper
id: act1/keeper
lamps: 0

Mabel: So you're the new keeper. I'm Mabel, and I kept this parlor for forty years.
Mabel: It has been dark for twelve of them. Every game still works, if someone will play it.

## In the small hours

Wicket: Oh. A person. Mabel said a new keeper was coming, but she's asleep.
```

- `# Title` starts a scene. The title is its row in The story so far.
- Straight after the title come `key: value` lines that say what the scene
  is and when it plays. Keys are lowercase.
- Then one line per page. Each page starts with the speaker's name and a
  colon, and keeps to three sentences, so it reads cleanly through a
  screen reader and can be skipped.
- `## In the small hours` starts the pages told instead between 1 and 5 in
  the morning, when Mabel is asleep and Wicket hosts. A scene without them
  waits for a sitting Mabel hosts.
- Lines starting with `>` are notes for writers.
- In a regular's file, `who`, `games` and `after` lines come before the
  first scene.

### Keys

| File | Key | Meaning |
| --- | --- | --- |
| `main.md` | `id` | Saved once heard. Never change or reuse one. |
| `main.md` | `lamps` | Lamps lit before it plays: a number, `all but one` or `all`. |
| `main.md` | `wing` | Also waits for a lit game in this category: `seasonal`, `action`, `puzzle`, `relaxing` (Cozy games) or `table`. |
| `regulars/*.md` | `id` | `side/<who>/<number>`, in order. |
| `doors.md` | `id`, `game` | `door/<game id>`, and the game's id. |
| `visits/*.md` | `holiday`, `game`, `after` | Before the first scene: the holiday, the visitor's game, and the main plot scene a player must hear first. |
| `visits/*.md` | `year` | The year it plays, or `any` for the stock visit, which plays in years with none written. |
| `visits/*.md` | `plays` | `arrival` (the first sitting in the run-up), `after the game` (after the player finishes the visitor's game), or `on the day`. Each year starts with an arrival. |
| `visits/*.md` | `id` | `visit/<holiday>/<year>/<number>`, or `visit/<holiday>/stock/<number>`. |
| `chapters/*.md` | `chapter`, `title` | Before the first scene: its number, counting from 1 in the order of `chapters::FILES`, and its title. |
| `chapters/*.md` | `id` | `chapter/<chapter>/<number>`, in order. |
| any scene file | `music` | The piece that plays while a scene is told. Before the first scene it sets the file's default. See Music below. |
| `talk.md` | `when` | `any`, or a part of the day, a season and a weekday, joined by commas: `night, winter`. Parts of the day are dawn, morning, afternoon, evening, night and small hours. |
| `talk.md` | `after` | Optional: a scene the player must have heard first, such as `act1/biscuit` for a line that names Biscuit. |

A visit runs while its holiday is near: from the holiday's run-up, the
same days the Happening today list shows it, to the day after. Its
scenes play in order, one at a sitting's start or one after the
visitor's game. The `on the day` scene plays on the day even if the
player missed some before it. While a visitor's scenes are still to come,
and on the day, the sitting favours the visitor's game.

Chapters play once the player has heard The keys, one scene a sitting,
at the sitting's start, in order. The first scene opens with the
chapter's number and title, and the lobby's Story row says a new
chapter is waiting. A new chapter file also goes in `FILES` in
`crates/wickets/src/chapters.rs`.

In `talk.md` each line under a heading is one opening line, without a
speaker's name: the game adds Mabel's, or Wicket's in the small hours.

## Music

Story mode has its own soundtrack. The sitting takes turns with three
pieces (theme, fireside and lamplit), and plays Wicket's small hours piece
from 1 to 5 in the morning. A scene plays its own piece while its pages
are read, then the sitting's music comes back.

A scene's piece comes from its own `music:` line, else the file's, else
its kind: `theme` for the main plot, doors and visits, `fireside` for the
regulars, `sunday` for chapters.

| Name | Mood |
| --- | --- |
| `theme` | The story's waltz: the lamps, the parlor coming back. |
| `fireside` | Mabel's: warm, brushed piano. Everyday life in the parlor. |
| `lamplit` | A folk tune: the regulars, the town, comings and goings. |
| `sunday` | Theo's: harp and cor anglais. Hopeful, a little wistful. |
| `small hours` | Wicket's: glass and celesta. Night, the dark years, the attic. |
| `echo` | The cellar and the Echo: tuning forks and playback. |
| `tender` | Felt piano and cello: apologies, letters, hard truths. |
| `jack` | Lantern Jack's mischievous waltz. |
| `party` | The whole band: opening nights, the keys, the ending. |

The pieces are built by `tools/story_music.py` into
`sound-specs/parlor.json`, then rendered with
`python tools/render_sounds.py parlor`. A new piece goes in that script,
in `STORY_SCENES` in `crates/wickets/src/music.rs`, and in this table.

## Speakers

A page may only be spoken by someone in `CAST` in
`crates/wickets/src/story_file.rs`. A new character goes there first.

## Checks

`cargo test -p wickets` reads every story file. It fails on a file the
game doesn't build in, a heading or key it can't read, a page with no
known speaker or more than three sentences, an id used twice, an id in
`shipped.txt` that no file has, or a scene that isn't in `shipped.txt` yet.
`python tools/check_text.py` holds every page to the in-game text rules.

A new file goes in `FILES` in `crates/wickets/src/story_file.rs`.
