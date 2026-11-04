//! Sound keys and where fireworks sit around the listener.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::rules::{Crowd, Kind, Launch, Pos};

pub const LAUNCH: &str = "liberty-lights/launch/whoosh";
pub const FUSE: &str = "liberty-lights/launch/fuse";
pub const TICK: &str = "liberty-lights/count/tick";
pub const GO: &str = "liberty-lights/count/go";
pub const COMBO: &str = "liberty-lights/cue/combo";
pub const UNDO: &str = "liberty-lights/cue/undo";
pub const AIM: &str = "liberty-lights/cue/aim";
pub const SAVED: &str = "liberty-lights/cue/saved";
pub const HINT: &str = "liberty-lights/cue/hint";
pub const SELECT: &str = "liberty-lights/cue/select";
pub const TWIST: &str = "liberty-lights/cue/twist";
/// Independence night's opening salute begins.
pub const SALUTE: &str = "liberty-lights/cue/salute";
pub const CHALLENGE: &str = "liberty-lights/music/challenge";
pub const NIGHT: &str = "liberty-lights/ambience/night";
/// The show's dynamic score (see `parlor_core::score`).
pub const SCORE: [parlor_core::Stem; 6] = parlor_core::score_stems!("liberty-lights");
pub const TITLE_MUSIC: &str = "liberty-lights/music/title";
pub const WIN: &str = "liberty-lights/music/win";
pub const LOSE: &str = "liberty-lights/music/lose";
pub const FINALE: &str = "liberty-lights/music/finale";

pub fn firework(kind: Kind) -> &'static str {
    match kind {
        Kind::Boom => "liberty-lights/fireworks/boom",
        Kind::Crackle => "liberty-lights/fireworks/crackle",
        Kind::Whistle => "liberty-lights/fireworks/whistle",
        Kind::Sparkle => "liberty-lights/fireworks/sparkle",
    }
}

pub fn crowd(crowd: Crowd) -> &'static str {
    match crowd {
        Crowd::Cheer => "liberty-lights/crowd/cheer",
        Crowd::Ooh => "liberty-lights/crowd/ooh",
        Crowd::Aah => "liberty-lights/crowd/aah",
        Crowd::Murmur => "liberty-lights/crowd/murmur",
    }
}

/// Where a launch place is heard: the original panned left and right to
/// 80 percent; this sits each place 1.5 m away (full volume) at that pan.
pub fn place(pos: Pos) -> Vec3 {
    match pos {
        Pos::Left => Vec3::new(-1.2, 0.9, 0.0),
        Pos::Center => Vec3::new(0.0, 1.5, 0.0),
        Pos::Right => Vec3::new(1.2, 0.9, 0.0),
    }
}

/// A firework bursting at its place.
pub fn burst(ctx: &mut Ctx<Parlor>, kind: Kind, pos: Pos) -> Sound {
    ctx.audio.play_at(firework(kind), place(pos), Bus::EFFECTS)
}

/// The player's launch: the mortar's thump, then the burst.
pub fn launch(ctx: &mut Ctx<Parlor>, l: Launch) {
    ctx.audio.play_at(LAUNCH, place(l.pos), Bus::EFFECTS);
    burst(ctx, l.kind, l.pos);
}

/// A show firework heard through the night fog: three times as far off
/// at the same side, and quieter.
pub fn fogged(ctx: &mut Ctx<Parlor>, l: Launch) {
    let near = place(l.pos);
    let at = Vec3::new(near.x * 3.0, near.y * 3.0, near.z);
    let thump = ctx.audio.play_at(LAUNCH, at, Bus::EFFECTS);
    ctx.audio.set_volume(thump, 0.35);
    let s = ctx.audio.play_at(firework(l.kind), at, Bus::EFFECTS);
    ctx.audio.set_volume(s, 0.45);
}

/// The crowd's reaction, louder the better the show.
pub fn react(ctx: &mut Ctx<Parlor>, accuracy: u32) {
    let s = ctx
        .audio
        .play(crowd(Crowd::for_accuracy(accuracy)), Bus::EFFECTS);
    ctx.audio
        .set_volume(s, 0.55 + 0.45 * accuracy.min(100) as f32 / 100.0);
}

/// A tick at a launch place, for aiming.
pub fn aim(ctx: &mut Ctx<Parlor>, pos: Pos) {
    ctx.audio.play_at(AIM, place(pos), Bus::EFFECTS);
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The night outside: music off, a quiet bed on.
pub fn night(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(NIGHT));
}
