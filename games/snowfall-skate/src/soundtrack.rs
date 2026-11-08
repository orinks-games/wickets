//! Snowfall Skate's music for Wicket's soundtrack: the title theme, the
//! pause music, and the skate's medley, played anew each time.

use hearken::{Performer, Track};
use parlor_core::soundtrack::Album;

use crate::music;

pub const ALBUM: Album = Album::new(crate::ID, crate::NAME);

/// The title theme's length, seconds.
pub const TITLE_S: f64 = 40.0;
/// The pause music's length, seconds.
pub const PAUSE_S: f64 = 24.0;

pub const MEDLEY: &str = "snowfall-skate/medley";

pub fn tracks() -> Vec<Track> {
    vec![
        ALBUM.title(TITLE_S),
        Track::live(MEDLEY, "A night on the ice", |v| {
            Box::new(music::Night::new(v)) as Box<dyn Performer>
        })
        .unlocked_by(music::every_key())
        .varied()
        .album(ALBUM.name)
        .about("The skate's medley of carols and a waltz, strung together anew each time."),
        ALBUM.pause(PAUSE_S),
    ]
}
