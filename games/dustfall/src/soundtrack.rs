//! Dustfall's music for Wicket's soundtrack: the title theme, the score
//! played anew each time, and the pause music.

use hearken::{Performer, Track};
use parlor_core::soundtrack::Album;

use crate::music;

pub const ALBUM: Album = Album::new(crate::ID, "Dustfall");

/// The title theme's length, seconds.
pub const TITLE_S: f64 = 48.0;
/// The pause music's length, seconds.
pub const PAUSE_S: f64 = 24.0;

pub const SCORE: &str = "dustfall/score";

pub fn tracks() -> Vec<Track> {
    vec![
        ALBUM.title(TITLE_S),
        Track::live(SCORE, "A night at the dome", |v| {
            Box::new(music::Performance::new(v)) as Box<dyn Performer>
        })
        .unlocked_by(music::every_key())
        .varied()
        .album(ALBUM.name)
        .about(
            "The night's score, built a bar at a time as the raiders come nearer, anew each play.",
        ),
        ALBUM.pause(PAUSE_S),
    ]
}
