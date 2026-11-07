//! Larkfield's music for Wicket's soundtrack: the title theme, supper and
//! the pause music. The hamlet's tune is made up live as the day goes
//! ([`crate::music`]), so it has no recording to list.

use hearken::Track;
use parlor_core::soundtrack::Album;

pub const ALBUM: Album = Album::new(crate::ID, "Larkfield");

/// The title theme's length, seconds.
pub const TITLE_S: f64 = 45.71;
/// The pause music's length, seconds.
pub const PAUSE_S: f64 = 24.0;

pub fn tracks() -> Vec<Track> {
    vec![ALBUM.title(TITLE_S), ALBUM.pause(PAUSE_S)]
}
