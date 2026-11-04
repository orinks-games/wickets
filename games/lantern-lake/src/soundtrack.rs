//! Lantern Lake's music for Wicket's soundtrack.
//!
//! The lake's song is its base and its ten voices, the loops a night joins
//! as lanterns are gathered. From the Soundtrack menu it is arranged anew
//! each play: which voices open, the order they join, where they drop out
//! and how it thins out all change, so no two plays are the same take.
//!
//! Each night the player paddles is also kept, as the loop each voice
//! joined on ([`Night`]), and listed in "Your Lantern Lake nights" to hear
//! again as it was ([`NightSong`]).

use hearken::{Audio, Bus, Performer, Sound, Stems, Track};
use parlor_core::soundtrack::Album;
use serde::{Deserialize, Serialize};

use crate::cues;

pub const ALBUM: Album = Album::new(crate::ID, "Lantern Lake");

/// The song's id on the soundtrack.
pub const SONG: &str = "lantern-lake/song";
/// Lantern Festival night's song, with temple bells for its last voice.
pub const FESTIVAL_SONG: &str = "lantern-lake/festival-song";

/// Each song voice's level, as on the lake.
const LEVEL: f32 = crate::screens::LAYER_VOLUME;

pub fn tracks() -> Vec<Track> {
    let mut festival: Vec<&str> = cues::SONG_LAYERS[..cues::SONG_LAYERS.len() - 1].to_vec();
    festival.push(cues::TEMPLE_BELLS);
    vec![
        ALBUM.title(64.0),
        Track::stems(
            SONG,
            "The night's song",
            Stems::new(cues::SONG_LOOP)
                .bed([cues::SONG_BASE])
                .voices(cues::SONG_LAYERS)
                .level(LEVEL),
        )
        .unlocked_by([cues::SONG_LAYERS[0]])
        .album(ALBUM.name)
        .hint("Gather a lantern on the lake.")
        .about("The song the lanterns sing, each voice joining in its own order."),
        Track::stems(
            FESTIVAL_SONG,
            "Lantern Festival song",
            Stems::new(cues::SONG_LOOP)
                .bed([cues::SONG_BASE])
                .voices(festival)
                .level(LEVEL),
        )
        .unlocked_by([cues::TEMPLE_BELLS])
        .album(ALBUM.name)
        .secret()
        .about("Lantern Festival night's song, with temple bells for its last voice."),
        ALBUM.pause(34.3),
    ]
}

/// Where the player's nights are kept, in the records.
pub const HISTORY_KEY: &str = "lantern-lake.nights";

/// The album the player's own nights are listed under.
pub const HISTORY_ALBUM: &str = "Your Lantern Lake nights";

/// One night's song as the player heard it: when, where, and the loop each
/// voice joined on. A few numbers, so no recording is kept.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Night {
    /// Counts up across every night kept, so each has its own track.
    pub n: u32,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    /// The night's name, such as "Night 1".
    pub mode: String,
    /// Lantern Festival night: temple bells for the last voice.
    pub festival: bool,
    /// The weather's sound under the song, if any.
    pub weather: Option<String>,
    /// The song loop each voice joined on, voice 1 first.
    pub joins: Vec<u32>,
    pub complete: bool,
}

impl Night {
    fn date(&self) -> parlor_core::Date {
        parlor_core::Date::new(self.year, self.month, self.day)
    }

    pub fn title(&self) -> String {
        format!(
            "{}, {}",
            self.mode,
            parlor_core::soundtrack::date_words(self.date())
        )
    }
}

/// Keeps a night that had any song, newest first.
pub fn keep_night(ctx: &mut hearken::Ctx<parlor_core::Parlor>, mut night: Night) {
    if night.joins.is_empty() {
        return;
    }
    let last = parlor_core::soundtrack::kept::<Night>(ctx, HISTORY_KEY)
        .first()
        .map_or(0, |n| n.n);
    night.n = last + 1;
    parlor_core::soundtrack::keep(ctx, HISTORY_KEY, night);
}

/// The player's own nights, as tracks open from the start.
pub fn history(ctx: &mut hearken::Ctx<parlor_core::Parlor>) -> Vec<Track> {
    parlor_core::soundtrack::kept::<Night>(ctx, HISTORY_KEY)
        .into_iter()
        .map(|night| {
            let title = night.title();
            let about = format!(
                "Your song from {}, {} voice{}{}, as you heard it.",
                parlor_core::soundtrack::date_words(night.date()),
                night.joins.len(),
                if night.joins.len() == 1 { "" } else { "s" },
                if night.complete { ", whole" } else { "" }
            );
            Track::live(
                format!("lantern-lake/night/{}", night.n),
                title,
                move |_| Box::new(NightSong::new(&night)) as Box<dyn Performer>,
            )
            .album(HISTORY_ALBUM)
            .open()
            .about(about)
        })
        .collect()
}

/// Loops to wait between two voices joining, at most, when a night is
/// played again: a long paddle between lanterns is shortened.
const MOST_LOOPS_APART: u32 = 2;
/// Loops the whole song plays after the last voice joins.
const WHOLE_LOOPS: u32 = 2;
const END_FADE_MS: u32 = 2500;
const STOP_FADE_MS: u32 = 600;

/// One night's song played again: the base, the weather, and each voice
/// joining on its loop, the gaps between them shortened.
pub struct NightSong {
    /// (key, loop) for each voice, in the order they join.
    voices: Vec<(&'static str, u32)>,
    weather: Option<String>,
    complete: bool,
    clock: f64,
    end_at: f64,
    started: Vec<Sound>,
    next: usize,
    ended: bool,
}

impl NightSong {
    pub fn new(night: &Night) -> NightSong {
        let mode = if night.festival {
            crate::rules::Mode::Festival
        } else {
            crate::rules::Mode::Free
        };
        let mut voices = Vec::new();
        let mut at = 0u32;
        let mut last = None;
        for (i, &join) in night.joins.iter().enumerate() {
            at = match last {
                None => join.min(1),
                Some(prev) => at + join.saturating_sub(prev).min(MOST_LOOPS_APART),
            };
            last = Some(join);
            let layer = (i + 1).min(crate::rules::SONG_LAYERS);
            voices.push((crate::screens::song_key(mode, layer), at));
        }
        let end_loops = voices.last().map_or(1, |v| v.1) + WHOLE_LOOPS;
        NightSong {
            voices,
            weather: night.weather.clone(),
            complete: night.complete,
            clock: 0.0,
            end_at: end_loops as f64 * cues::SONG_LOOP,
            started: Vec::new(),
            next: 0,
            ended: false,
        }
    }

    /// The loop each voice joins on, for tests.
    pub fn plan(&self) -> Vec<u32> {
        self.voices.iter().map(|v| v.1).collect()
    }

    /// Seconds the song runs before it fades.
    pub fn length_s(&self) -> f64 {
        self.end_at
    }

    fn start_due(&mut self, audio: &mut Audio) {
        while let Some(&(key, at)) = self.voices.get(self.next) {
            if (at as f64) * cues::SONG_LOOP > self.clock + 1e-6 {
                break;
            }
            let s = audio.start_loop(key, None, Bus::MUSIC);
            audio.set_volume(s, LEVEL);
            self.started.push(s);
            self.next += 1;
        }
    }
}

impl Performer for NightSong {
    fn start(&mut self, audio: &mut Audio) {
        let base = audio.start_loop(cues::SONG_BASE, None, Bus::MUSIC);
        audio.set_volume(base, 0.7);
        self.started.push(base);
        if let Some(bed) = &self.weather {
            let s = audio.start_loop(bed, None, Bus::MUSIC);
            audio.set_volume(s, 0.3);
            self.started.push(s);
        }
        self.start_due(audio);
    }

    fn update(&mut self, audio: &mut Audio, dt: f64) -> bool {
        if self.ended {
            return false;
        }
        let before = (self.clock / cues::SONG_LOOP).floor();
        self.clock += dt;
        let after = (self.clock / cues::SONG_LOOP).floor();
        if after > before {
            self.start_due(audio);
        }
        if self.clock >= self.end_at {
            for s in self.started.drain(..) {
                audio.stop(s, END_FADE_MS);
            }
            if self.complete {
                audio.play(cues::MUSIC_COMPLETE, Bus::MUSIC);
            }
            self.ended = true;
            return false;
        }
        true
    }

    fn stop(&mut self, audio: &mut Audio) {
        for s in self.started.drain(..) {
            audio.stop(s, STOP_FADE_MS);
        }
        self.ended = true;
    }
}
