//! Rooftop Garden's music for Wicket's soundtrack.
//!
//! The garden's song is played live, the way the garden plays it: its own
//! tune first, then new tunes made up as it goes ([`crate::song`]). Each
//! play from the Soundtrack menu draws its own seed, so the made-up tunes,
//! which plants sing, the order they join and their places in the stereo
//! field all change, in the garden's own key and tempo. There is no recording of the piece to keep: the
//! song only exists as it is played.
//!
//! Each session in the player's garden is kept too, as its song's seed and
//! the parts that sang ([`Session`]), and listed in "Your Rooftop Garden
//! songs" to hear again: the same made-up tunes, the same plants.

use hearken::audio::Start;
use hearken::{Audio, Bus, Performer, Sound, Track, Variety};
use parlor_core::soundtrack::Album;

use crate::cues;
use crate::song::{Part, Song, BAR, BARS};
use serde::{Deserialize, Serialize};

pub const ALBUM: Album = Album::new(crate::ID, "Rooftop Garden");

/// The song's id on the soundtrack.
pub const SONG: &str = "rooftop-garden/song";

/// The parts a soundtrack play draws its singers from.
pub const SINGERS: [Part; 10] = [
    Part::Tomato,
    Part::Squash,
    Part::Bean,
    Part::Sunflower,
    Part::Lavender,
    Part::Mint,
    Part::Strawberry,
    Part::Chilli,
    Part::Rose,
    Part::Jasmine,
];

pub fn tracks() -> Vec<Track> {
    vec![
        ALBUM.title(60.0),
        Track::live(SONG, "The garden's song", |v| {
            Box::new(GardenSong::new(v)) as Box<dyn Performer>
        })
        .unlocked_by([cues::SONG_BASE])
        // The garden makes up new tunes as it plays, so every play differs.
        .varied()
        .album(ALBUM.name)
        .hint("Grow a plant until it sings.")
        .about(
            "The garden's own tune, then new ones made up as it plays. Every play is a new song.",
        ),
        ALBUM
            .piece(
                "music/shed",
                "The potting shed",
                "Visit the potting shed.",
                20.9,
            )
            .about("The music in the seed shed, where you browse seeds and collections."),
        ALBUM.pause(25.3),
    ]
}

/// How the pad sits under the parts, as in the garden.
const BASE_VOLUME: f32 = 0.6;
/// How long the last bar rings before the song ends.
const RING_OUT_S: f64 = 3.0;
const STOP_FADE_MS: u32 = 600;

/// One singer: its part, level and place, and what it is singing.
struct Singer {
    part: Part,
    volume: f32,
    pan: f32,
    /// This bar's sound, and the last bar's still ringing out.
    now: Option<Sound>,
    tail: Option<Sound>,
}

/// One live play of the garden's song.
pub struct GardenSong {
    song: Song,
    singers: Vec<Singer>,
    /// How many singers have joined.
    joined: usize,
    base: Option<Sound>,
    base_tail: Option<Sound>,
    /// Playback speed, the same for every part: as recorded.
    pitch: f32,
    /// Verses this play runs.
    verses: usize,
    /// Verses begun.
    verse: usize,
    /// Seconds left of the ring-out, once the last verse is done.
    ending: Option<f64>,
}

impl GardenSong {
    pub fn new(mut v: Variety) -> GardenSong {
        let mut parts = SINGERS.to_vec();
        v.shuffle(&mut parts);
        // Four to six plants sing.
        parts.truncate(4 + v.below(3) as usize);
        let singers = parts
            .into_iter()
            .map(|part| Singer {
                part,
                volume: 0.65 + 0.3 * v.unit(),
                pan: v.spread(0.7),
                now: None,
                tail: None,
            })
            .collect();
        GardenSong {
            song: Song::new(v.next_u64()),
            singers,
            joined: 1 + v.below(2) as usize,
            base: None,
            base_tail: None,
            pitch: 1.0,
            // Five to seven verses: a minute to a minute and a half.
            verses: 5 + v.below(3) as usize,
            verse: 0,
            ending: None,
        }
    }

    /// A session's song played again: its seed, so the same made-up
    /// tunes, and its parts in the order they joined, one more each verse.
    pub fn replay(seed: u64, parts: &[Part], verses: usize) -> GardenSong {
        let mut v = Variety::new(seed);
        let singers = parts
            .iter()
            .map(|&part| Singer {
                part,
                volume: 0.8,
                pan: v.spread(0.7),
                now: None,
                tail: None,
            })
            .collect();
        GardenSong {
            song: Song::new(seed),
            singers,
            joined: 1,
            base: None,
            base_tail: None,
            pitch: 1.0,
            verses: verses.clamp(2, 7),
            verse: 0,
            ending: None,
        }
    }

    /// The parts singing in this play, in the order they join.
    pub fn parts(&self) -> Vec<Part> {
        self.singers.iter().map(|s| s.part).collect()
    }

    /// How many verses this play runs.
    pub fn verses(&self) -> usize {
        self.verses
    }

    fn begin(&self, audio: &mut Audio, key: &str, from: f64, volume: f32, pan: f32) -> Sound {
        audio.start_with(
            key,
            Bus::MUSIC,
            Start {
                volume,
                pan,
                pitch: self.pitch,
                from_s: from,
                ..Start::ONCE
            },
        )
    }

    /// The top of a bar: every part starts what the song gives it here.
    fn play_bar(&mut self, audio: &mut Audio) {
        if self.song.bar() == 0 {
            self.verse += 1;
            if self.verse > self.verses {
                self.ending = Some(RING_OUT_S);
                return;
            }
            // A plant joins with each new verse until all are in.
            if self.verse > 1 && self.joined < self.singers.len() {
                self.joined += 1;
            }
        }
        if let Some((key, from)) = self.song.cue(Part::Base, self.base.is_none()) {
            let s = self.begin(audio, &key, from, BASE_VOLUME, 0.0);
            let last = self.base.replace(s);
            if let Some(old) = std::mem::replace(&mut self.base_tail, last) {
                audio.stop(old, 300);
            }
        }
        for i in 0..self.joined.min(self.singers.len()) {
            let (part, joining, volume, pan) = {
                let s = &self.singers[i];
                (s.part, s.now.is_none(), s.volume, s.pan)
            };
            if let Some((key, from)) = self.song.cue(part, joining) {
                let sound = self.begin(audio, &key, from, volume, pan);
                let s = &mut self.singers[i];
                let last = s.now.replace(sound);
                if let Some(old) = std::mem::replace(&mut s.tail, last) {
                    audio.stop(old, 300);
                }
            }
        }
    }

    fn sounds(&mut self) -> Vec<Sound> {
        let mut out: Vec<Sound> = self.base.take().into_iter().collect();
        out.extend(self.base_tail.take());
        for s in &mut self.singers {
            out.extend(s.now.take());
            out.extend(s.tail.take());
        }
        out
    }
}

impl Performer for GardenSong {
    fn start(&mut self, audio: &mut Audio) {
        self.song.restart();
        self.play_bar(audio);
    }

    fn update(&mut self, audio: &mut Audio, dt: f64) -> bool {
        if let Some(left) = self.ending.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                for s in self.sounds() {
                    audio.stop(s, STOP_FADE_MS);
                }
                return false;
            }
            return true;
        }
        // The song keeps time at the play's tempo.
        if self.song.tick(dt * self.pitch as f64) {
            self.play_bar(audio);
        }
        true
    }

    fn stop(&mut self, audio: &mut Audio) {
        for s in self.sounds() {
            audio.stop(s, STOP_FADE_MS);
        }
        self.ending = Some(0.0);
    }
}

/// About how long one play runs, in seconds, at the recorded tempo.
pub fn length_s(verses: usize) -> f64 {
    verses as f64 * BARS as f64 * BAR + RING_OUT_S
}

/// Where the player's garden songs are kept, in the records.
pub const HISTORY_KEY: &str = "rooftop-garden.songs";

/// The album the player's own garden songs are listed under.
pub const HISTORY_ALBUM: &str = "Your Rooftop Garden songs";

/// One session's song: its seed and the parts that sang, in the order they
/// joined. A few numbers, so no recording is kept.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Session {
    /// Counts up across every session kept, so each has its own track.
    pub n: u32,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    /// The garden's own day count when the session ended.
    pub garden_day: u32,
    pub seed: u64,
    /// Each part's id ([`Part::id`]), in the order it joined.
    pub parts: Vec<String>,
    /// Verses the session played.
    pub verses: u32,
}

impl Session {
    fn date(&self) -> parlor_core::Date {
        parlor_core::Date::new(self.year, self.month, self.day)
    }

    pub fn title(&self) -> String {
        format!(
            "Garden day {}, {}",
            self.garden_day,
            parlor_core::soundtrack::date_words(self.date())
        )
    }

    pub fn parts(&self) -> Vec<Part> {
        self.parts
            .iter()
            .filter_map(|id| Part::from_id(id))
            .collect()
    }
}

/// Keeps a session in which anything sang, newest first.
pub fn keep_session(ctx: &mut hearken::Ctx<parlor_core::Parlor>, mut session: Session) {
    if session.parts().is_empty() {
        return;
    }
    let last = parlor_core::soundtrack::kept::<Session>(ctx, HISTORY_KEY)
        .first()
        .map_or(0, |s| s.n);
    session.n = last + 1;
    parlor_core::soundtrack::keep(ctx, HISTORY_KEY, session);
}

/// The player's own garden songs, as tracks open from the start.
pub fn history(ctx: &mut hearken::Ctx<parlor_core::Parlor>) -> Vec<Track> {
    parlor_core::soundtrack::kept::<Session>(ctx, HISTORY_KEY)
        .into_iter()
        .map(|session| {
            let parts = session.parts();
            let about = format!(
                "Your garden's song from {}, with {} plant{} singing.",
                parlor_core::soundtrack::date_words(session.date()),
                parts.len(),
                if parts.len() == 1 { "" } else { "s" }
            );
            let (seed, verses) = (session.seed, session.verses as usize);
            Track::live(
                format!("rooftop-garden/session/{}", session.n),
                session.title(),
                move |_| Box::new(GardenSong::replay(seed, &parts, verses)) as Box<dyn Performer>,
            )
            .album(HISTORY_ALBUM)
            .open()
            .about(about)
        })
        .collect()
}
