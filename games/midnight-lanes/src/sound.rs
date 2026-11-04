//! Sound keys, where each sound sits, and the sound check's catalog.

use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

use crate::rules::CENTRE;

pub const STEP: &str = "midnight-lanes/step";
pub const AIM_TONE: &str = "midnight-lanes/aim-tone";
pub const SWING: &str = "midnight-lanes/swing-rise";
pub const RELEASE: &str = "midnight-lanes/release";
pub const BALL_ROLL: &str = "midnight-lanes/ball-roll";
pub const GUTTER: &str = "midnight-lanes/gutter";
pub const PIN_LIGHT: &str = "midnight-lanes/pin-hit-light";
pub const PIN_HEAVY: &str = "midnight-lanes/pin-hit-heavy";
pub const PINSETTER: &str = "midnight-lanes/pinsetter";
pub const BALL_RETURN: &str = "midnight-lanes/ball-return";
pub const STRIKE: &str = "midnight-lanes/strike-sting";
pub const PUMPKIN_LIGHT: &str = "midnight-lanes/pumpkin-hit-light";
pub const PUMPKIN_HEAVY: &str = "midnight-lanes/pumpkin-hit-heavy";
pub const GRIN: &str = "midnight-lanes/grin-carved";
pub const SPARE: &str = "midnight-lanes/spare-bell";
pub const AMBIENCE: &str = "midnight-lanes/alley-ambience";
pub const TITLE_MUSIC: &str = "midnight-lanes/title-music";

/// The swing tone's pitch at the start, at the top, and at the end of the
/// swing. It rises to the top and falls after it.
pub const SWING_LOW: f32 = 0.7;
pub const SWING_HIGH: f32 = 1.4;
/// The ball's loudness at the foul line and at the pins.
pub const ROLL_NEAR: f32 = 0.35;
pub const ROLL_FAR: f32 = 1.0;
/// Pins at least this many fall with the heavy clatter.
pub const HEAVY: u8 = 5;

/// Where a board sits across the lane, for panning: left of centre is to
/// the left.
pub fn board_place(board: f64) -> Vec3 {
    Vec3::new(
        ((board - CENTRE as f64) / CENTRE as f64 * 3.0) as f32,
        2.0,
        0.0,
    )
}

pub fn play(ctx: &mut Ctx<Parlor>, key: &str) -> Sound {
    ctx.audio.play(key, Bus::EFFECTS)
}

pub fn play_at(ctx: &mut Ctx<Parlor>, key: &str, board: f64) -> Sound {
    ctx.audio.play_at(key, board_place(board), Bus::EFFECTS)
}

/// The swing tone's pitch `t` seconds into a swing that tops out at `top`.
pub fn swing_pitch(t: f64, top: f64) -> f32 {
    let rise = (t / top).clamp(0.0, 1.0) as f32;
    let fall = ((t - top) / top).clamp(0.0, 1.0) as f32;
    SWING_LOW + (SWING_HIGH - SWING_LOW) * (rise - fall)
}

/// The ball's loudness at `progress` down the lane.
pub fn roll_volume(progress: f64) -> f32 {
    ROLL_NEAR + (ROLL_FAR - ROLL_NEAR) * progress.clamp(0.0, 1.0) as f32
}

pub fn title_music(ctx: &mut Ctx<Parlor>) {
    ctx.audio.set_bed(None);
    ctx.audio.play_music(Some(TITLE_MUSIC), true);
}

/// The alley after closing, music off.
pub fn alley_bed(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
    ctx.audio.set_bed(Some(AMBIENCE));
}

/// Every sound, with its name and meaning, for the sound check.
pub fn catalog() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            STEP,
            "Step: a soft shoe on the approach. You moved one board, and it sounds from where you stand.",
        ),
        (
            AIM_TONE,
            "Aim tone: a soft high tone. It sounds from where the ball would cross the pins with a clean release.",
        ),
        (
            SWING,
            "Swing tone: a hum that rises to the top of the swing, then falls. Let go at the top.",
        ),
        (
            RELEASE,
            "Release: a thump as the ball meets the lane.",
        ),
        (
            BALL_ROLL,
            "Rolling ball: a low rumble that moves across with the ball and grows louder as it nears the pins.",
        ),
        (
            GUTTER,
            "Gutter: a hollow rattle. The ball left the lane and will hit no pins.",
        ),
        (
            PIN_LIGHT,
            "Light clatter: a few pins falling.",
        ),
        (
            PIN_HEAVY,
            "Heavy clatter: most of the rack falling at once.",
        ),
        (
            PINSETTER,
            "Pinsetter: a mechanical sweep and clunk as the deck is cleared and pins are set.",
        ),
        (
            BALL_RETURN,
            "Ball return: a rolling rumble and a knock. Your ball is back and you can roll.",
        ),
        (
            STRIKE,
            "Strike sting: a short jukebox flourish. All ten pins with the first ball.",
        ),
        (SPARE, "Spare bell: a bright bell. The second ball cleared the rest."),
        (
            PUMPKIN_LIGHT,
            "Pumpkins falling, light: a hollow squash. On Pumpkin night, a few pumpkins went down.",
        ),
        (
            PUMPKIN_HEAVY,
            "Pumpkins falling, heavy: a long hollow tumble. On Pumpkin night, five or more went down.",
        ),
        (
            GRIN,
            "Grin carved: a knife scrape and a low chord. A strike on Pumpkin night.",
        ),
    ]
}
