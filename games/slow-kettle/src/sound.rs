//! Sound keys, the kettle's song, and the sound check's catalog.

use hearken::{Bus, Ctx, Sound};
use parlor_core::Parlor;

use crate::rules::{Stage, BOIL, ROOM};
use crate::teas::{Cup, Tune};

pub const TITLE_MUSIC: &str = "slow-kettle/music/title";
pub const SHOP_MUSIC: &str = "slow-kettle/music/shop";
pub const CLOSING: &str = "slow-kettle/music/closing";
pub const HOLIDAY: &str = "slow-kettle/music/holiday";
pub const MIDNIGHT: &str = "slow-kettle/music/midnight";

pub const HOUSE: &str = "slow-kettle/ambience/house";
pub const RAIN: &str = "slow-kettle/ambience/rain";

pub const RUMBLE: &str = "slow-kettle/kettle/rumble";
pub const HISS: &str = "slow-kettle/kettle/hiss";
pub const WHISTLE: &str = "slow-kettle/kettle/whistle";
pub const KETTLE_ON: &str = "slow-kettle/kettle/click";
pub const POUR: &str = "slow-kettle/kettle/pour";

pub const TICK: &str = "slow-kettle/steep/tick";
pub const LIFT: &str = "slow-kettle/steep/lift";
pub const TIMER: &str = "slow-kettle/steep/timer";
pub const ICE: &str = "slow-kettle/steep/ice";
pub const CUP_POUR: &str = "slow-kettle/steep/cup";

pub const BELL: &str = "slow-kettle/door/bell";
pub const STEPS: &str = "slow-kettle/door/steps";
pub const STRANGER: &str = "slow-kettle/door/stranger";

pub const SHELF: &str = "slow-kettle/shelf/move";
pub const TIN: &str = "slow-kettle/shelf/tin";

pub const PERFECT: &str = "slow-kettle/verdict/perfect";
pub const LOVELY: &str = "slow-kettle/verdict/lovely";
pub const KIND: &str = "slow-kettle/verdict/kind";
pub const COINS: &str = "slow-kettle/shop/coins";
pub const BUY: &str = "slow-kettle/shop/buy";
pub const KEEPSAKE: &str = "slow-kettle/cue/keepsake";
pub const HINT: &str = "slow-kettle/cue/hint";
pub const NOPE: &str = "slow-kettle/cue/nope";

/// The set-down sound of each cup, by its id.
pub fn cup(cup: &Cup) -> &'static str {
    match cup.id {
        "mug" => "slow-kettle/cup/mug",
        "clay" => "slow-kettle/cup/clay",
        "glass" => "slow-kettle/cup/glass",
        "porcelain" => "slow-kettle/cup/porcelain",
        _ => "slow-kettle/cup/white",
    }
}

/// The music box loop of a tune.
pub fn tune(tune: &Tune) -> &'static str {
    match tune.id {
        "river" => "slow-kettle/musicbox/river",
        "lanterns" => "slow-kettle/musicbox/lanterns",
        _ => "slow-kettle/musicbox/rain",
    }
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The tea house: its bed, and the music box if it is on.
pub fn house(ctx: &mut Ctx<Parlor>, bed: &str, tune: Option<&Tune>) {
    ctx.audio.set_bed(Some(bed));
    match tune {
        Some(t) => ctx.audio.play_music(Some(self::tune(t)), true),
        None => ctx.audio.play_music(None, false),
    }
}

/// How loud each voice of the kettle's song is at `temp`: the rumble,
/// the hiss and the whistle.
pub fn song_levels(temp: f64) -> [f32; 3] {
    let t = temp.clamp(ROOM, BOIL);
    let ramp = |from: f64, to: f64| ((t - from) / (to - from)).clamp(0.0, 1.0) as f32;
    let rumble = ramp(ROOM, 50.0) * (1.0 - 0.7 * ramp(70.0, 95.0));
    let hiss = ramp(60.0, 92.0);
    let whistle = ramp(97.0, BOIL);
    [rumble * 0.8, hiss * 0.7, whistle * 0.5]
}

/// The kettle's three loops, mixed by its heat.
#[derive(Debug, Default)]
pub struct KettleSong {
    voices: Option<[Sound; 3]>,
}

impl KettleSong {
    pub fn start(&mut self, ctx: &mut Ctx<Parlor>, temp: f64) {
        self.stop(ctx);
        let voices = [RUMBLE, HISS, WHISTLE].map(|k| ctx.audio.start_loop(k, None, Bus::EFFECTS));
        self.voices = Some(voices);
        self.follow(ctx, temp);
    }

    pub fn follow(&mut self, ctx: &mut Ctx<Parlor>, temp: f64) {
        let Some(voices) = self.voices else {
            return;
        };
        let levels = song_levels(temp);
        let rise = ((temp - ROOM) / (BOIL - ROOM)).clamp(0.0, 1.0) as f32;
        for (i, (sound, level)) in voices.into_iter().zip(levels).enumerate() {
            ctx.audio.set_volume(sound, level);
            if i < 2 {
                ctx.audio.set_pitch(sound, 0.9 + 0.2 * rise);
            }
        }
    }

    pub fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(voices) = self.voices.take() {
            for s in voices {
                ctx.audio.stop(s, 400);
            }
        }
    }

    pub fn playing(&self) -> bool {
        self.voices.is_some()
    }
}

/// The sound of the kettle at each stage, for the sound check.
pub fn stage_sound(stage: Stage) -> &'static str {
    match stage {
        Stage::Quiet | Stage::Rumble | Stage::Bubbles => RUMBLE,
        Stage::Hiss | Stage::Rolling => HISS,
        Stage::Boil => WHISTLE,
    }
}

/// Every cue with its name and meaning, for the sound check.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            BELL,
            "Door bell: a small brass bell. A guest comes in.",
        ),
        (STEPS, "Footsteps: soft steps on the wooden floor, coming to the counter."),
        (SHELF, "Shelf tick: a soft wooden tick as you move along the shelf or the cups."),
        (TIN, "Tin lid: a tea tin opens. Your tea goes in the pot."),
        (KETTLE_ON, "Kettle switch: a click. The kettle is on."),
        (
            RUMBLE,
            "Rumble: the kettle's low song as it starts to warm, up to about 65 degrees.",
        ),
        (
            HISS,
            "Hiss: the kettle's rushing song from about 80 degrees, rising toward the boil.",
        ),
        (
            WHISTLE,
            "Whistle: a gentle whistle. The kettle is at the boil, 100 degrees.",
        ),
        (POUR, "Pour: water from the kettle onto the leaves. The steep begins."),
        (TICK, "Clock tick: one second of steeping."),
        (
            TIMER,
            "Sand timer chime: the steep has reached what the tea wants. Only once you own the sand timer.",
        ),
        (LIFT, "Leaves lifted: a drip and a swish. The steep is over."),
        (CUP_POUR, "Cup filled: tea poured from the pot into one cup."),
        (ICE, "Ice: cubes clink in the glass, for an iced tea."),
        (PERFECT, "Perfect cup: three warm rising notes."),
        (LOVELY, "Lovely cup: two warm notes."),
        (KIND, "Kind cup: one soft note. The guest is still pleased."),
        (COINS, "Coins: coins on the counter, for the cup and any tip."),
        (NOPE, "Soft thud: nothing to do just now."),
    ]
}
