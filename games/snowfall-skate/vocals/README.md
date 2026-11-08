# Sung parts for Snowfall Skate

These are the carol sections the game's brass band plays at the Frost Fair, and the new year's song, with their words, for recording sung tracks to lay over the band.

`tools/snowfall_skate_sounds.py` writes every file here from the same melodies the game plays, so the singing lines up with the band exactly. Run it again after changing a melody.

## The files

Each section has two files:

- `<section>.mid`: the melody, one note per syllable, at the game's tempo and meter, with each syllable as a MIDI lyric event on its note. Most singing synths can import a MIDI file with its lyrics.
- `<section>.txt`: the same syllables in order, in case the lyrics need pasting in by hand.

A syllable ending in `-` runs on into the next note of the same word ("Christ- mas"). A lone `-` holds the syllable before it over one more note.

| Section | Tune | Tempo | Meter | Length |
| --- | --- | --- | --- | --- |
| `tree-a` | O Christmas Tree, first half | 120 | 3/4 | 24 beats |
| `tree-b` | O Christmas Tree, second half | 120 | 3/4 | 24 beats |
| `wish` | We Wish You a Merry Christmas | 150 | 3/4 | 24 beats |
| `king-a` | Good King Wenceslas, first half | 112 | 4/4 | 32 beats |
| `king-b` | Good King Wenceslas, second half | 112 | 4/4 | 36 beats |
| `still-a` | Silent Night, first half | 96 | 3/4 | 36 beats |
| `still-b` | Silent Night, second half | 96 | 3/4 | 36 beats |
| `year` | Auld Lang Syne | 100 | 4/4 | 32 beats |

Each carol's pickup note is moved to the end of the section before it, so every section starts on its first downbeat. That is why `tree-a` starts on "Christ" and ends on "Not", and `wish` starts on "wish" and ends on "We". The game's waltz has no words and has no file here.

All are in G major.

## Rendering

1. Import each `.mid` into the singing synth, with its lyrics.
2. Keep the tempo and the start exactly as in the file: the first note is on beat 1, with no lead-in.
3. Sing it as a small village choir or a single warm voice, gently: it plays under a brass band on an ice rink.
4. Export each as a WAV the length of the section plus about 2 seconds of ring-out, named after its section, such as `tree-a.wav`.

## Where the recordings go

Put the WAVs in `games/snowfall-skate/vocals/recorded/`, and say so on the pull request. The game then needs a small change to play them: a sung track joins the band's lead whenever that section plays in the band style (at the Frost Fair, and the new year's song at midnight). That change and the Ogg encoding are done in the game's repository once the recordings are in.
