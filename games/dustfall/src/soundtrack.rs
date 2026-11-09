//! Dustfall's music for Wicket's soundtrack: the title theme and the pause
//! music.

use hearken::Track;
use parlor_core::soundtrack::Album;

pub const ALBUM: Album = Album::new(crate::ID, "Dustfall");

/// The title theme's length, seconds.
pub const TITLE_S: f64 = 48.0;
/// The pause music's length, seconds.
pub const PAUSE_S: f64 = 24.0;

pub fn tracks() -> Vec<Track> {
    vec![ALBUM.title(TITLE_S), ALBUM.pause(PAUSE_S)]
}
