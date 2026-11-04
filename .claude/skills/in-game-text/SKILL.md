---
name: in-game-text
description: Use when writing or reviewing any text a player hears or reads in an audio game: menus, help lines, spoken status, tutorials, help.md, results, errors and story.
---

# In-game text for audio games

Players hear this text through a screen reader or TTS, usually at high speed,
often while a game sound plays, and they can cut it off with the next key.
They cannot glance back at it. Write so that the first few words carry the
point and every line still makes sense when heard alone, out of order, or
repeated by R.

After you write or edit any prose, also run the unslop skill
(`anthropic-skills:unslop`) on it.

## Wicket's mechanics this skill assumes

- `ctx.say(text)` interrupts whatever is speaking. Use it for the direct
  answer to a keypress.
- `ctx.say_queued(text)` waits for current speech. Use it for a follow-up,
  such as the focused row after a result. The next keypress may cut it off,
  so never put the only copy of important information there.
- `MenuCore` speaks a row as `end_sentence(label)` plus `"2 of 5."` when
  positions are on. F1 reads the row's `.help(..)`, or the menu's
  `with_intro_help(..)` when the row has none. `resume_with(result)` says the
  result, then queues the focused row: `"Via Ames added."`, `"Plan: Ames. 2 of 5."`.
- Name keys with `ctx.control_name(ParlorAction::X)`, and wrap it in
  `hearken::text::sentence_start(..)` when it opens a sentence.
- Long text (story, tutorial, debrief) goes in `Pages`: Enter next page, R
  repeats, Up and Down step back and forward a page, Escape skips. One
  `String` per page.
- `help.md` is split on `## ` headings by `parlor_core::Help`. Each heading
  is a topic the player can jump to, so each section must stand alone.
  Inside a topic each paragraph and list item is one page, read alone with
  Up and Down, so keep paragraphs to two or three sentences and make each
  one make sense on its own.
- Use `end_sentence` instead of appending "." yourself; ".." is read as "dot dot".

## 1. Menu items and their help lines

Labels:
- Two to four words. Lead with the word that tells rows apart.
  Before: `Start a new game of Safecracker` / After: `New game`
- Verbs for actions, nouns for places. `Crack the next safe`, `Best times`, `Settings`.
- No type words. The menu already sounds like a menu.
  Before: `Settings menu`, `Quit button` / After: `Settings`, `Quit`
- No trailing period, colon or ellipsis in a plain label; `end_sentence` adds the period.
- A row that holds a value reads `Name: value`, and Left/Right read back
  only the new value. Before: `Music volume is currently set to 70 percent` /
  After: `Music volume: 70 percent`
- Locked rows keep their name and say why in the label or help.
  After: `Time attack, locked` with help `Crack safe 3 to open time attack.`
- Use the same label for the same thing on every screen. If the title says
  `Challenges`, the lobby, help and results say `Challenges` too.

Help lines (`.help(..)`):
- One sentence, under about 20 words, ending in a period. Say what the row
  does or contains, not that it is a row.
  Before: `This option lets you view your best times.` /
  After: `Your best times and scores for every safe.`
- Add what is new to the label; do not restate it.
  Before: `Practice dial: practise the dial.` /
  After: `Endless single wheels with nothing to lose: learn the clunk.`
- For a row that leaves the game or loses progress, say where it goes.
  After: `Return to the game list.`

Intro help (`with_intro_help`): the keys for this menu, in one line, using
the verb for this screen. After: `Up and Down choose a safe, Enter plays it, Escape goes back.`

## 2. Spoken status and announcements

- Answer every keypress, in words or a sound. Silence reads as a crash.
- Front-load the changed fact. Context comes after, or not at all.
  Before: `You have now successfully set wheel number 2.` / After: `Wheel 2 set. 1 to go.`
- Event lines: one to eight words. `Mine.` `Sonar recharging.` `Out of the current.`
- Do not narrate what the player just did if the sound already confirmed it;
  say the result. Before: `You pressed Space. The wheel is set.` / After: `Wheel 2 set.`
- When nothing changed, say so and why. `Already all stop.` `No hints left for this safe.`
- The status key (S) reads a fixed order every time, most urgent first, as a
  comma list ending in a period. Keep the order stable so players learn
  where to stop listening. After: `Wheel 2 of 4, dial at 37, turn left, 1 strike, 2 hints left.`
- Pair every earcon with words the first time it plays, then rely on the
  sound. After: `That hollow rattle was a false gate. Ignore it: only the deep clunk is the tumbler.`
- Warnings name the threat and the action, in that order, and nothing else.
  Before: `Be careful, it sounds like someone might be approaching!` /
  After: `Footsteps! The guard is coming. When they stop, keep still.`
- Never say something only with pitch, pan or timing. S or a spoken line must cover it.

## 3. Instructions that name controls

- Never hard-code a key in spoken text. Build it from `ctx.control_name(..)`.
  Before: `"Press Space to set the wheel."` /
  After: `format!("{} sets the wheel.", sentence_start(&ctx.control_name(ParlorAction::Primary)))`
- Key first, then effect: `{p} sets the wheel.` reads faster than `To set the wheel, press {p}.`
  Use "press" only when the sentence needs a verb.
- Group pairs: `{l} and {r} turn the dial one number down or up.`
- Write "press", "hold", "let go". Never "click", "tap", "see", "look", "above", "below", "on screen".
- Engine keys are fixed and may be named literally: F1 help, F2 stops speech, Escape pauses or goes back.
- help.md may name default keys, with this line once per file: `These are
  the default keys. If you change them in Settings, the game's spoken instructions follow your bindings.`
- Controls lists use `Key: effect.` one per line or bullet, in the order the
  player needs them. After: `Space: set the wheel.`

## 4. Tutorials and help documents

Tutorials (`Pages`):
- One idea per page, three to five sentences. A page is heard, then gone.
- Teach in the order the player meets things. Rules they meet on safe 5 go
  in a prompt on safe 5, not on page 1.
- Each page says what to listen for and what key answers it.
  After: `When it reaches the number, the tumbler drops with a deep, solid clunk, just after the click. Then {p} sets the wheel.`
- Let the player act soon. After two or three pages, hand over control and
  prompt the next step in play.
- Do not write "as mentioned", "see above", "earlier", "the next section". R repeats only this page, and a player who skims back with Up may hear pages out of order.
- Offer a no-stakes practice mode and point to it.

help.md:
- First section `## About the game`: what the player is doing and how they
  win or lose, in under 80 words.
- One `## ` heading per topic, named by the player's question: `## Hints`,
  `## The night guard`, `## Scoring and best times`. Not `## Misc` or `## Notes`.
- Each section stands alone. Repeat a key or term rather than point elsewhere.
- Define every game word on first use in its section: `That is a strike.`
- Describe each sound in plain words a listener can match: `a hollow metal clang`, `a bright double chime`.
- End with `## Controls`. Short paragraphs; no tables, no nested lists, no bold for meaning.

## 5. Results and failure messages

Results:
- Order: outcome, score, record, next step. Each its own short sentence.
  Before: `Congratulations!!! You did it! Your final score was 640 points which is a new personal best!` /
  After: `Safe 3 open. 640 points, a new best. Enter for the next safe.`
- Say what changed in records: `a new best`, `2 seconds off your best`, `streak 4 days`.
- Long debriefs go in `Pages`, the headline on page 1.

Failures and errors:
- Say what happened, why, and what to do. No blame, no apology, no drama.
  Before: `GAME OVER. You failed.` /
  After: `Caught. The guard heard the dial while he listened. Enter tries the safe again.`
- Wrong input: name the rule broken. Before: `Invalid move.` /
  After: `Right number, wrong way: wheel 3 must be reached turning left.`
- Counted mistakes say the count and the limit: `Strike 2 of 3.`
- Do not use "invalid", "error", "failed to" or codes in player text.
- Destructive actions confirm in words, naming what is lost and both keys.
  After: `Erase all Safecracker records? Enter erases, Escape keeps them.`

## 6. Story text

- Story may use richer language than UI text, but keep pages short and
  skippable, and keep rules out of story pages; teach them in play.
- Name the speaker first: `The curator: Nobody has opened that vault in forty years.`
- Avoid dialect spelling, made-up spellings and puns that TTS will mangle.
  Read invented names aloud with a screen reader before keeping them.
- Give a one-page summary for anything longer than a few pages, so a player
  who skipped still knows the goal.

## 7. Speech-specific rules

Length and order:
- Sentences under about 20 words; spoken lines in play under about 12.
- Most important word first, in every line. Players stop listening once they have what they need.
- One fact per sentence in play. Two short sentences beat one with "and".

Punctuation (NVDA, JAWS and VoiceOver differ at default settings):
- Use periods and commas for pauses. A colon is fine mid-line; some readers say "colon" at higher punctuation levels, so do not end a line on one.
- No em-dashes, en-dashes, parentheses, ellipses, slashes, asterisks, `&`,
  `+`, `~`, `#` or emoji in spoken text. Some readers say them aloud, some skip them silently.
  Before: `Hull 80% (2 hits left) / air 40s` / After: `Hull 80 percent, 2 hits left. Air 40 seconds.`
- One exclamation mark at most, for real alarms. Never `!!` or `?!`.
- No ALL CAPS words; readers may spell them letter by letter.

Numbers and units:
- Digits for quantities: `3 hints`, `Strike 2 of 3`, `120 metres`.
- Spell units out: `metres`, `seconds`, `percent`, `degrees`. Not `m`, `s`, `%`, `deg`.
- Durations in words: `1 minute 30 seconds`, not `1:30` (it may read as a clock time or "1 30").
- Signs in words: `minus 5`, `5 points lost`, not `-5` (a hyphen is often silent) or `+20`.
- Ordinals: `first`, `second`, `third` for order; avoid `1st`, `2nd`.
- No Roman numerals: `Level 2`, not `Level II`.
- Handle plurals with a branch, never `hint(s)`: `1 hint` / `{h} hints`.

Abbreviations and words:
- Spell out on first use; avoid `Dr.`, `St.`, `No.`, `approx.`, `e.g.`, `etc.`, `vs`.
- Watch homographs that change sound with meaning: read, lead, wind, wound, tear, live, close, bow.
  Rephrase if the context is short. Before: `Wind the dial.` / After: `Turn the dial.`
- Do not misspell words to force a pronunciation; braille users read the misspelling.
- Do not dot or space acronyms (`H P`, `H.P.`). Prefer the full words in play: `hull`, not `HP`.

Interruption and repeat:
- Assume any line can be cut after its first three words.
- Anything critical goes in the `say`, not a `say_queued` tail.
- Each prompt must stand alone for R: no "this", "that one", "again" without a noun.
  Before: `Do that again, but faster.` / After: `Spin back ten, then step forward one at a time.`
- Do not queue more than two lines after a keypress. Put the rest behind S or H.

## 8. Localization-friendly writing

- Build each spoken line with one `format!` and named placeholders, not by
  joining fragments; word order differs between languages.
  Before: `"Wheel ".to_string() + &n.to_string() + " set"` / After: `format!("Wheel {n} set.")`
- Never build a noun phrase from an adjective variable and a noun variable; gender and case vary.
- No idioms, slang or wordplay in instructions. Story can keep them, with a note for translators.
- Budget about 30 percent more length; translated speech takes longer to hear.

## Review checklist

Run each line through these, then run `anthropic-skills:unslop` on the prose.

1. Does the first word or two carry the point?
2. Is every key built from `ctx.control_name`, with `sentence_start` at a sentence start?
3. Are labels two to four words, with no "button", "menu" or final period?
4. Does every `.help(..)` add something the label does not say, in one sentence?
5. Does every keypress get a response, and every earcon a spoken line the first time?
6. Is critical information in `ctx.say`, not only in a queued line?
7. Does each prompt make sense alone after R, with no "above", "this" or "again"?
8. Any symbols, dashes, parentheses, ALL CAPS, `1:30`, `-5`, `%`, abbreviations or Roman numerals in spoken text?
9. Are plurals branched, and lines built with one `format!`?
10. Is each thing called by one name across labels, help.md, tutorial, status and sound check?
11. Do results lead with the outcome and failures say what happened, why and what to do?
12. Does each `## ` section of help.md stand alone, and does each tutorial page hold one idea?
13. No em-dashes, no marketing tone, no praise inflation ("amazing", "epic").
