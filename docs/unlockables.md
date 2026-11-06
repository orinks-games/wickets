# Unlockables across Wicket's

Every game gets an Unlocks row on its title, built on the shared
`parlor_core::unlocks` module. Nevermore Manor is the first to use it, with
63 unlocks. This plan proposes a set for every other game, so each one gives
a player something to chase after the first clear.

## How it works

- A game lists its unlocks once: an id, a group, a name, the rule that earns
  it, and what it does. The id is saved, so names and rules can change later.
- The game decides when a rule is met and calls `Unlocks::earn_all`. It says
  what was earned at the end of the round, with a chime: "Unlocked: Diary page
  3 and Swift."
- The title row reads "Unlocks, 12 of 40". It opens one row per group, such
  as "Music, 2 of 6". Earned entries are listed by name. Locked ones read
  "Name, locked", and Enter or F1 says the rule. Nothing is hidden: a player
  can always hear what is left and how to get it.
- Enter on an earned entry uses it. It can read a story page, play a sound or
  a tune, switch a modifier on or off, or say what a feat was for.
- A `sync` step on the title earns anything older saves already qualify for,
  so existing players get their unlocks the first time they open the game.

## Groups every game can use

Not every game needs every group. Pick the ones that fit.

- **Modes**: a harder or longer way to play, opened by a milestone. Many games
  already have these (Nightmare, Night skyline, Endless rail, Deep start).
  Listing them as unlocks gathers them in one place.
- **Modifiers** (Nevermore Manor calls them curses): rules you switch on for
  more score, each earned by meeting it once in play. Each one adds a quarter
  to the score. They never apply to challenges, so challenges stay the same
  for everyone.
- **Story**: short pages of the game's world, one every few levels. They need
  no new sounds, and they give a reason to keep going.
- **Sound gallery**: every sound the player has met, with what it means. It
  doubles as a reference for players who learn by ear.
- **Music**: the game's tunes and beds, earned by reaching where they play.
- **Feats**: harder goals that need no new content, such as no hints, no
  misses, a fast clear or a long streak.
- **Cosmetic sounds**: a different set of footsteps, a bell or a paddle
  splash. Choose one with Enter. These are cheap to add and players notice
  them.

## Per game

Counts are targets, not limits.

| Game | Groups | About |
|------|--------|-------|
| Liberty Lights | Modes, Shows, Feats, Music | Shows: named fireworks finales, each earned by a perfect level in a band of five, then playable as a listen-only show. Feats: perfect levels, early-launch catches, a 20-level run without a miss. Modes: a hard show with more overlap once level 20 is cleared. About 30. |
| Harvest Feast | Recipes, Modifiers, Feats, Story | Recipes: new dishes earned by serving a station's dish perfectly a number of times. Each adds a new sound to the kitchen. Modifiers: rush hour (faster orders), burnt edges (smaller windows). Story: notes from the family who come to dinner. About 35. |
| Winter Wonderland | Modes, Gift sounds, Bells, Feats, Story | Gift sounds: new gift kinds heard in Gift Sort. Bells: alternative midnight bells, chosen with Enter. Story: letters from the village, one per shift and night. About 35. |
| Basket Run | Baskets, Modifiers, Feats, Sound gallery | Baskets: cosmetic catch sounds. Modifiers: windy meadow, more bees. Feats: golden-egg streaks, no-mud runs. About 30. |
| Neon Rail: Black Sky Run | Archive, Upgrades, Feats, Music | The archive and workshop already hold progress, so they move under the shared menu. Upgrades: new upgrade cards added to the pool as the player extracts. Archive: pilot logs, one per extraction. About 40. |
| Safecracker | Safes, Dials, Feats, Story | Dials: cosmetic dial and clunk sets, such as an old bank vault or a hotel safe. Story: the case file behind each safe. Feats: no false gates touched, time-attack records. About 35. |
| Belfry | Peals, Bells, Feats, Story | Peals: named methods, earned by solving rows, then played back. Bells: different towers' bell sets. Story: the old ringer's notebook. About 30. |
| Sonar Dive | Wrecks, Creatures, Modes, Feats | Wrecks: each wreck found is logged with a page of its history. Creatures: a sound gallery of sea life met on dives. Modes: Deep start, already in the game, plus a no-sonar-recharge challenge dive. About 40. |
| Tightrope | Crossings, Modes, Modifiers, Feats, Story | Story: the city below, one page per crossing. Modifiers: no gust warning, pigeon flocks. Night skyline moves into Modes. About 30. |
| Signal Box | Routes, Trains, Feats, Story | Trains: a sound gallery of each train class. Routes: new lines opened by shifts survived. Story: the signalman's log. About 30. |
| Midnight Lanes | Balls, Lanes, Feats | Balls: cosmetic roll sounds. Lanes: alleys with different acoustics. Feats: strikes, spares, turkeys and a 300 game. About 30. |
| Soft Landing | Moons, Landers, Feats, Story | Landers: handling variants with their own engine sound. Moons are already progress, and each one gets a page of mission notes. Feats: fuel-saving and touchdown speed. About 35. |
| Pantry Mouse | Pantries, Cats, Modifiers, Feats | Cats: a gallery of each cat's sounds and habits. Modifiers: more cats, creakier floors. About 30. |
| Freewheel | Courses, Bikes, Bells, Feats | Bikes and courses become unlocks earned by time-trial times. Bells: cosmetic bike bells. Feats: clean rides with no near misses. About 35. |
| Riverboat Dice | Landings, Dice, Feats, Story | Landings: the river towns, one per landing banked. Dice: cosmetic dice and cup sounds. Story: tales from the passengers. About 30. |
| Fireside Patience | Card backs, Fire sounds, Feats | Fire sounds: hearth beds, chosen with Enter. Feats: wins without undo, fast wins and streaks. About 25. |
| Lantern Lake | Songs, Paddles, Journal | No failing and no pressure: the journal already tallies lanterns. Songs: each night's finished song, kept to replay. Paddles: cosmetic paddle and boat sounds. About 25. |
| Tide Pools | Aquarium, Shells, Journal | The aquarium is already a collection, so it becomes a group. Rare creatures are earned on later tides. About 40. |
| Rooftop Garden | Plants, Layers, Journal | Plants: new species earned by growing others to full. Each adds a new layer of music. Layers: finished garden songs to replay. About 30. |

The cozy games (Lantern Lake, Tide Pools, Rooftop Garden) get no
modifiers and no feats that need speed. Their unlocks are collections and
songs, earned by playing at any pace.

## Order of work

1. Games whose content is already collections: Tide Pools, Neon Rail and
   Sonar Dive. They mostly need ids, rules and the menu.
2. Games with levels and no story yet: Safecracker, Tightrope, Liberty Lights
   and Soft Landing. Add story pages, feats and one cosmetic group.
3. The rest, one game per pull request, each with its own tests: earned on
   the right event, saved, `sync` from old saves, and the menu rows.

Each game needs the shared label and the "Unlocks" title row, a short
"Unlocks" section in its help.md, and tests in its own test binary. New
sounds go through `sound-specs` and `tools/render_sounds.py` like any other
sound.
