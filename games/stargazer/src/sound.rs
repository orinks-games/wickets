//! Every sound key the game plays, how it plays them, and the sound check's
//! list of them.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::{Parlor, Season};

pub const MUSIC_TITLE: &str = "stargazer/music/title";
pub const MUSIC_SHOWER: &str = "stargazer/music/shower";
pub const MUSIC_HOME: &str = "stargazer/music/home";

pub const AMBIENCE_WINTER: &str = "stargazer/ambience/winter";
pub const AMBIENCE_SPRING: &str = "stargazer/ambience/spring";
pub const AMBIENCE_SUMMER: &str = "stargazer/ambience/summer";
pub const AMBIENCE_AUTUMN: &str = "stargazer/ambience/autumn";

pub const CHIMES: [&str; 3] = [
    "stargazer/star/chime_1",
    "stargazer/star/chime_2",
    "stargazer/star/chime_3",
];
pub const BRIGHT: &str = "stargazer/star/bright";
pub const FAINT: &str = "stargazer/star/faint";

pub const STEP: &str = "stargazer/point/step";
pub const EDGE: &str = "stargazer/point/edge";

pub const TRACE: &str = "stargazer/trace/star";
pub const NOT_YET: &str = "stargazer/trace/not_yet";
pub const NOTHING: &str = "stargazer/cue/nothing";
pub const HINT: &str = "stargazer/cue/hint";
pub const GUIDE: &str = "stargazer/cue/guide";

pub const RISE: &str = "stargazer/sky/rise";
pub const ALL_TRACED: &str = "stargazer/sky/all_traced";

pub const SHIMMER: &str = "stargazer/wonder/shimmer";
pub const WONDER_FOUND: &str = "stargazer/wonder/found";

pub const MOON: &str = "stargazer/moon/glow";
pub const MOON_GREET: &str = "stargazer/moon/greet";

pub const FOUND_STAR: &str = "stargazer/hunt/found";

pub const METEOR: &str = "stargazer/meteor/streak";
pub const FIREBALL: &str = "stargazer/meteor/fireball";
pub const WISH: &str = "stargazer/meteor/wish";

/// A constellation's own little tune, by its id.
pub fn motif(id: &str) -> String {
    format!("stargazer/motif/{id}")
}

/// The season's night sounds on the hill.
pub fn ambience(season: Season) -> &'static str {
    match season {
        Season::Winter => AMBIENCE_WINTER,
        Season::Spring => AMBIENCE_SPRING,
        Season::Summer => AMBIENCE_SUMMER,
        Season::Autumn => AMBIENCE_AUTUMN,
    }
}

/// A cue from the centre.
pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

/// A cue from the sky: `pan` left to right, `ahead` front to back, with
/// its volume and pitch. The listener sits at the centre, facing ahead.
pub fn play_from(
    ctx: &mut Ctx<Parlor>,
    key: &str,
    pan: f32,
    ahead: f32,
    gain: f32,
    pitch: f32,
) -> Sound {
    let s = ctx
        .audio
        .play_at(key, Vec3::new(pan, ahead, 0.0), Bus::EFFECTS);
    ctx.audio.set_volume(s, gain.clamp(0.0, 1.0));
    ctx.audio.set_pitch(s, pitch.clamp(0.5, 2.0));
    s
}

/// The title's music, beds off.
pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(MUSIC_TITLE), true);
}

/// The hilltop: the season's night sounds, music off, the listener centred.
pub fn hill(ctx: &mut Ctx<Parlor>, season: Season) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(ambience(season)));
    ctx.audio.set_listener(Vec3::ZERO, 0.0);
}

/// How the sound check plays an entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Play {
    /// Once, from the centre.
    Once,
    /// From the left, then from the right.
    Sides,
    /// Rising in pitch, as a star above the point sounds.
    Height,
    /// Sweeping left to right, as a shooting star does.
    Sweep,
    /// Through the music slot.
    Music,
}

/// One row of the sound check.
pub struct Entry {
    pub name: String,
    pub meaning: String,
    pub key: String,
    pub play: Play,
}

fn e(name: &str, meaning: &str, key: &str, play: Play) -> Entry {
    Entry {
        name: name.to_string(),
        meaning: meaning.to_string(),
        key: key.to_string(),
        play,
    }
}

/// Every sound in the game, with its name and what it means.
pub fn catalog() -> Vec<Entry> {
    let list = vec![
        e("Star chime", "A star twinkling near the listening point. It rings from its side of the sky, louder for a brighter star.", CHIMES[0], Play::Sides),
        e("Star chime, second voice", "Another star. Stars ring one of 3 chimes, which all mean the same.", CHIMES[1], Play::Sides),
        e("Star chime, third voice", "The third star chime.", CHIMES[2], Play::Sides),
        e("Bright star", "One of the brightest stars of all, such as Sirius or Vega: a fuller, ringing chime.", BRIGHT, Play::Sides),
        e("Faint star", "A faint star in no pattern: a tiny, soft tick of a chime.", FAINT, Play::Sides),
        e("A star above the point", "A chime higher in pitch comes from a star above the listening point; lower, from below it.", CHIMES[0], Play::Height),
        e("Listening point step", "The point moves one step. It is a little higher in pitch the higher the point is.", STEP, Play::Once),
        e("Edge of the sky", "The point is on the horizon or straight overhead, and goes no further.", EDGE, Play::Once),
        e("Star traced", "A star of a constellation is traced. The notes climb as the trace goes on.", TRACE, Play::Once),
        e("Not yet", "That star is not the next one to trace, or there is nothing to do there.", NOT_YET, Play::Once),
        e("Empty sky", "Nothing on this step of the sky.", NOTHING, Play::Once),
        e("Hint", "A hint, spoken with the way to the next star in steps, from that star's side.", HINT, Play::Sides),
        e("Guide tone", "When the guide is on, it pulses from the next star's direction, faster as you near it.", GUIDE, Play::Sides),
        e("Constellation rises", "A new constellation has risen into tonight's sky.", RISE, Play::Once),
        e("Every constellation traced", "All of tonight's constellations are traced.", ALL_TRACED, Play::Once),
        e("Shimmer", "Something hidden is near: a wonder such as a nebula or a cluster. Rest on it and press the main action.", SHIMMER, Play::Sides),
        e("Wonder found", "A wonder is found and written in your journal.", WONDER_FOUND, Play::Once),
        e("The full moon", "The full moon, low in the east, on full moon nights.", MOON, Play::Sides),
        e("Moon greeted", "You greeted the full moon.", MOON_GREET, Play::Once),
        e("Star found", "In a star hunt challenge, the star you were looking for.", FOUND_STAR, Play::Once),
        e("Shooting star", "A shooting star streaks across the sky. Press the main action while it passes to make a wish.", METEOR, Play::Sweep),
        e("Fireball", "A brighter, slower shooting star.", FIREBALL, Play::Sweep),
        e("Wish made", "Your wish is made on a shooting star.", WISH, Play::Once),
    ];
    list
}
