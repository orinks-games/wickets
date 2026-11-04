# Templates

Copy one of these into a new file, then fill it in. The format is in
README.md, and the cast and open threads are in BIBLE.md.

## A chapter

Save as `chapters/NN-short-name.md`, add it to the end of `FILES` in
`crates/wickets/src/chapters.rs` and to `FILES` in
`crates/wickets/src/story_file.rs`, and add each id to `shipped.txt`.

```
> Chapter N. One or two lines on what happens and which open thread it
> picks up.

chapter: N
title: Short title
> Optional: the chapter's music, if not sunday. See Music in README.md.
music: sunday

# First scene's title
id: chapter/N/1
> Optional: this scene's own music.
music: tender

Mabel: First page. Three short sentences at most.
Wicket: Second page.

# Second scene's title
id: chapter/N/2

Theo: A page.
```

Three to six scenes. Each plays at the start of its own sitting, so each
should stand alone for a player who comes back days later.

## A holiday visit

For a new year of an existing visitor, add scenes to the end of their
file in `visits/`. For a new visitor, save as `visits/<holiday>.md`, add it
to `FILES` in `crates/wickets/src/visits.rs` and in
`crates/wickets/src/story_file.rs`, and add them to the visitors table in
BIBLE.md. Add each id to `shipped.txt`.

```
> Who the visitor is, and what they do in their game.

holiday: halloween
game: haunted-hallways
after: act1/wicket
music: jack

# The visitor arrives
id: visit/halloween/YEAR/1
year: YEAR
plays: arrival

Wicket: A page.

# Something happens after a game
id: visit/halloween/YEAR/2
year: YEAR
plays: after the game

Mabel: A page.

# The day itself
id: visit/halloween/YEAR/3
year: YEAR
plays: on the day

Mabel: A page.

# The stock arrival
id: visit/halloween/stock/1
year: any
plays: arrival

Mabel: A page that works in any year.
```

A new visitor's file needs a stock visit with `year: any`, which plays in
years with no visit of their own.

## A new door

When a game is added to the Long Hall, add a scene to `doors.md`:

```
# A door that wasn't there
id: door/<game id>
game: <game id>

Mabel: A page about the new door.
```
