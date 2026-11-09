# Agent instructions

## Writing rule: no accessibility or audio marketing talk

This applies to everything you write: READMEs, store and site descriptions,
release notes, changelogs, forum posts (audiogames.net and similar), commit
messages, PR text, and docs.

- Never describe a game or app as audio-first, blind-first, accessibility-first,
  truly accessible, designed with accessibility at the forefront, crafted for
  keyboard and screen reader use, specifically optimized for screen reader
  users, or anything that means the same thing.
- Exception for games: a game may say once, plainly, that it is an audio game
  for blind and visually impaired players (for example "An audio trucking game
  for blind and visually impaired players."). Say it one time, in the opening
  description, and nothing beyond that.
- Skip audio puffery too: "rich and immersive soundscapes", "a game designed to
  be heard", "sounds like you're really there", "your eyes don't matter, only
  your ears do", and the like.
- Why: the audience already takes accessibility and audio for granted. Saying
  so is filler. Describe what the thing actually does instead: its features,
  content, controls, and what changed.
- Facts are fine when they are specific and useful, for example "works with
  NVDA and JAWS" in a requirements list or a note about which keys open a menu.

## Writing rule: player text reads like a game manual

Player text is everything a player hears or reads: spoken lines, menu
labels and their help, tutorials, `help.md`, results, the sound check, and
the README that ships in the zip. Write it the way established audio game
manuals are written (Freight Fate's player manual is the house example):
plain verbs, the key and then what it does, one fact a sentence, and
nothing about the interface itself.

- Keys do plain things: open, close, start, stop, choose, move, play, take,
  send, read, say, report. They never answer, ask, wait, want, keep, hand
  or let. Before: "Up and Down choose, Enter answers, Escape keeps them
  waiting at the gate." After: "Up and Down choose a reply, Enter gives it,
  Escape goes back and the caller waits at the gate."
- Interface parts are not characters. A row, a page, a pile, a meadow or a
  stream does not wait, ask, answer, want or signal; the game and its
  characters do. Before: "the meadow waits: only Enter goes on." After:
  "Play pauses while a page is open. Enter closes it."
- Name the thing, not its place in the menu: never "the one you are on",
  "the row you are on", "where it says", "where it asked" or "the page
  above". Before: "Enter plays the one you are on." After: "Enter plays the
  chosen sound."
- Key first, then its effect, one key a sentence, in the order the player
  needs them. In a controls list: "Space: take the chosen count." In prose:
  "Space takes the chosen count." Use "press", "hold" and "let go" only when
  the sentence needs the verb.
- Say what happens, not that the game will tell you. Drop "says so", "and
  says what it means", "with its name and what it means", "tells you",
  "lets you" and "you hear" when the fact can stand on its own. Before:
  "Enter plays it and says what it means." After: "Enter plays it." Before:
  "Sound check plays each sound with its name and what it means." After:
  "Sound check plays the sounds you need on the trail."
- Nothing about the medium: no ears, listening, hearing, audio, speech or
  screen readers as a feature, and never "by ear". Describe the game.
- No filler openers or hedges: "note that", "keep in mind", "remember",
  "simply", "just", "as mentioned", "see above", "as you may have noticed".
  Say the fact.
- A sound is described only in the sound check. `help.md` and spoken lines
  name a cue by what it marks ("the freeze cue", "the strike cue"), never by
  how it sounds. Help explains gameplay, goals, scoring and controls.
- No timing numbers in spoken lines: "a moment", "straight away", "soon",
  not "0.2 seconds" or "10 s". `help.md` may give a rule's clock in whole
  seconds or minutes when the number is the rule.
- Short, plain sentences: under about 20 words in help, under about 12 in
  play, one fact each. Digits for counts, words for units and ranges ("2 to
  5", "metres", "Left and Right"), plurals branched in code, no dashes,
  parentheses or symbols in spoken text.
- One name for each thing everywhere: the label on the title is the word in
  `help.md`, the tutorial, the status line, the sound check and the results.
- A key that does nothing in a game says nothing: no "not a valid key", no
  flavour line.

`tools/check_text.py` flags the phrases above it can catch mechanically
(rule `manual-phrase`); the in-game-text skill and the unslop skill carry
the rest. Edit existing text lightly, and leave lines Orinks or Codex wrote
as they are: report a wording problem instead of rewriting it.

## Writing rule: no AI-isms

The unslop skill (`anthropic-skills:unslop`) is the full catalogue, and
every piece of prose goes through it before it ships. These are the habits
that show up in text written for Wicket's, with the fix for each. They
apply to player text, release notes, changelogs, README, PR text and docs.

- The negation reframe: "not X, but Y", "it's not about X, it's about Y",
  "not just X", "more than just X". Say the positive thing once.
- The countdown: "No clock, no score, just you and the lake." Name what the
  mode has, or name the one thing it lacks in a plain sentence: "There is no
  clock."
- Rule of three for its own sake: "stirring, flipping and pulling dishes",
  "calm, quiet and slow". Use the number of items the thing has.
- Fragments for drama: "Not the tool. The team.", "One pile. Two seats.",
  "That's it." Write a sentence.
- Mood adverbs and poetic nouns: softly, gently, quietly, a hush, a glow,
  a beat, a whisper of. In play they are noise; in a blurb they are
  puffery. Say what the thing is or does.
- Praise inflation in results: "Perfect!", "Well done!", "Amazing!",
  "Great job!". Results state the outcome and the number; a character may
  praise in their own voice, once.
- Flavour lines for keys that do nothing ("the stream waits", "nothing to
  do just now") and reassurance filler ("take your time", "no way to fail",
  "enjoy") outside a line that carries a rule. Unused keys stay silent; a
  practice mode's help says what it does not score, once.
- Chatbot leftovers: "Let's", "Welcome to", "Here's", "Simply", "Feel free
  to", "Note that", "Remember", "You'll notice", "You already know".
- Throat-clearing and signposting: "In this game, you will", "The goal is
  to", "As you progress", "At its core", "Ultimately". Start with the
  first fact.
- Significance inflation and vague vocabulary: delve, tapestry, testament,
  pivotal, robust, seamless, immersive, intricate, nuanced, vibrant,
  crucial, elevate, journey, experience (as a noun for the game). Use the
  plain word or cut.
- False agency: a meadow that waits, a pile that wants, a key that answers,
  a sound that "tells a story". Name the actor and the mechanism.
- Elegant variation: the pile is the pile in every line, never "the heap"
  in the next one; a regular is called by their name, not "your opponent",
  "your rival" and "the other player" in turn.
- Formatting tells in docs and release notes: Title Case headings, bold
  inline headers in lists, emoji, em-dashes, colon reveals ("The result:
  ..."), a "Conclusion" or "In summary" section, a closing recap.
- Hedges and meta: "it's worth noting", "it is important to", "generally
  speaking", "arguably", "in other words", "as mentioned above".
- Tense and address: present tense, "you" for the player, no "will" for
  what the game does now. Before: "The game will tell you when the round is
  over." After: "Last pebble. You lose the round."

`tools/check_text.py` flags the fixed phrases above it can catch (rule
`ai-phrase`). The rest is judgment: read the line as a player hears it at
speed, and cut what it does not need.
