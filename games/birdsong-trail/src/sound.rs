//! Sound keys, where sounds sit, and a small player for songs on menus.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::{Parlor, Season};

use crate::birds::{Note, Species, Voice};
use crate::rules::NOTE_GAP;
use crate::trails::{Ground, Trail};

pub const TITLE_MUSIC: &str = "birdsong-trail/music/title";
pub const TURN: &str = "birdsong-trail/turn";
pub const BUMP: &str = "birdsong-trail/bump";
pub const BEFRIEND: &str = "birdsong-trail/befriend";
pub const NEW_SONG: &str = "birdsong-trail/new-song";
pub const AGAIN: &str = "birdsong-trail/again";
pub const ARRIVE: &str = "birdsong-trail/arrive";
pub const FOUND: &str = "birdsong-trail/found";
pub const WINGS: &str = "birdsong-trail/wings";
pub const BED_HILL: &str = "birdsong-trail/bed/hill";
pub const BED_DAWN: &str = "birdsong-trail/bed/dawn";

/// A bird voice's note: "birdsong-trail/voice/whistle/low". A flock call
/// has one sound for every note.
pub fn voice_key(voice: Voice, note: Note) -> String {
    if voice.is_call() {
        format!("birdsong-trail/voice/{}", voice.slug())
    } else {
        format!("birdsong-trail/voice/{}/{}", voice.slug(), note.slug())
    }
}

/// Your own whistled note.
pub fn you_key(note: Note) -> String {
    format!("birdsong-trail/you/{}", note.slug())
}

pub fn step_key(ground: Ground) -> String {
    format!("birdsong-trail/step/{}", ground.slug())
}

/// The sound at the trail's end, which loops there.
pub fn end_key(trail: Trail) -> String {
    format!("birdsong-trail/end/{}", trail.slug())
}

/// The trail's ambience bed.
pub fn bed_key(trail: Trail, season: Season) -> String {
    let _ = season;
    format!("birdsong-trail/bed/{}", trail.slug())
}

/// A bird's note from where it sits, at its own pitch.
pub fn bird_note(ctx: &mut Ctx<Parlor>, species: &Species, note: Note, pos: Option<Vec3>) -> Sound {
    let key = voice_key(species.voice, note);
    let s = match pos {
        Some(p) => ctx.audio.play_at(&key, p, Bus::EFFECTS),
        None => ctx.audio.play(&key, Bus::EFFECTS),
    };
    ctx.audio.set_pitch(s, species.pitch);
    s
}

/// Your note, in the middle of your head.
pub fn your_note(ctx: &mut Ctx<Parlor>, note: Note) -> Sound {
    ctx.audio.play(&you_key(note), Bus::EFFECTS)
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn play_vol(ctx: &mut Ctx<Parlor>, key: &str, volume: f32) -> Sound {
    let s = ctx.audio.play(key, Bus::EFFECTS);
    ctx.audio.set_volume(s, volume);
    s
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// Music off and an ambience bed on, for play.
pub fn bed(ctx: &mut Ctx<Parlor>, key: &str) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(key));
}

pub fn v3(p: (f32, f32)) -> Vec3 {
    Vec3::new(p.0, p.1, 0.0)
}

/// Plays a run of sounds a note apart, on a screen with no rules of its
/// own, such as the field guide and the sound check.
#[derive(Default)]
pub struct Tune {
    queue: Vec<(f64, String, f32)>,
    playing: Vec<Sound>,
}

impl Tune {
    pub fn new() -> Tune {
        Tune::default()
    }

    /// Stops what was playing and queues a song: each key at its pitch.
    pub fn start(&mut self, ctx: &mut Ctx<Parlor>, keys: Vec<(String, f32)>) {
        self.stop(ctx);
        self.queue = keys
            .into_iter()
            .enumerate()
            .map(|(i, (k, p))| (i as f64 * NOTE_GAP, k, p))
            .collect();
        self.tick(ctx, 0.0);
    }

    /// A bird's song, notes at its pitch.
    pub fn song(&mut self, ctx: &mut Ctx<Parlor>, species: &Species, notes: &[Note]) {
        let keys = notes
            .iter()
            .map(|n| (voice_key(species.voice, *n), species.pitch))
            .collect();
        self.start(ctx, keys);
    }

    /// One sound now, stopping the tune.
    pub fn one(&mut self, ctx: &mut Ctx<Parlor>, key: &str, bus: Bus) {
        self.stop(ctx);
        let s = ctx.audio.play(key, bus);
        self.playing.push(s);
    }

    pub fn tick(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let mut due = Vec::new();
        self.queue.retain_mut(|(wait, key, pitch)| {
            *wait -= dt;
            if *wait <= 1e-9 {
                due.push((key.clone(), *pitch));
                false
            } else {
                true
            }
        });
        for (key, pitch) in due {
            let s = ctx.audio.play(&key, Bus::EFFECTS);
            ctx.audio.set_pitch(s, pitch);
            self.playing.push(s);
        }
    }

    pub fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        self.queue.clear();
        for s in self.playing.drain(..) {
            ctx.audio.stop(s, 80);
        }
    }
}
