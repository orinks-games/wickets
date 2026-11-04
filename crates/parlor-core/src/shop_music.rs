//! Music for shops: the menus where a player buys, fits or plants
//! something, such as Slow Kettle's shop or Neon Rail's workshop.
//!
//! A shop calls [`start`] from its `enter` and [`stop`] from its `exit`.
//! Stopping leaves the music slot empty, so the screen underneath brings
//! its own music back when it resumes: a title screen plays its theme
//! again, the tea house its music box, and a garden with no music stays
//! quiet. While a shop is open, the pause screen plays over it as usual.

use hearken::Ctx;

use crate::Parlor;

/// Starts the shop's own loop, cross-fading from whatever played before.
pub fn start(ctx: &mut Ctx<Parlor>, key: &str) {
    ctx.audio.play_music(Some(key), true);
}

/// Fades the shop's loop out on leaving.
pub fn stop(ctx: &mut Ctx<Parlor>) {
    ctx.audio.play_music(None, false);
}
