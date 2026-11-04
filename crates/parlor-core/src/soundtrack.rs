//! Wicket's soundtrack: every game's music, unlocked by hearing it in play
//! and played from the launcher's Soundtrack menu (Hearken's
//! `SoundtrackScreen`, one album per game).
//!
//! The launcher lists the tracks once with [`register`]; [`Parlor`]'s
//! `Game::soundtrack` hands them to Hearken. A game builds its own tracks
//! with an [`Album`], so each one carries the game's name and a hint in
//! the same words. A track is a sound key the game already plays: the first
//! time it starts in play, the track unlocks. A track plays as the player
//! hears it in the game, in its own key and mix; a piece built from stems,
//! such as a game's dynamic score, comes together in a new order each play,
//! as it does in play.
//!
//! A seasonal piece, or one from a game the player may not have reached, is
//! `secret`: neither listed nor counted until it is heard, so no menu names
//! a surprise before its season.
//!
//! The Sound check plays every cue out of play, so it unlocks nothing: the
//! shared title stops listening while it is open ([`quiet`]).
//!
//! # The player's own songs
//!
//! A game whose music is made in play, such as a night's song on the lake,
//! can keep each one the player heard as a few numbers in their records
//! ([`keep`]): a date, a seed, the order its voices joined. Its history
//! tracks are built from those and listed in an album of their own, open
//! from the start, so the player can hear their own night again.

use std::sync::OnceLock;

use hearken::{Ctx, Stems, Track};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{Date, Parlor, Records};

static TRACKS: OnceLock<fn() -> Vec<Track>> = OnceLock::new();
/// Builds the player's own songs from their records.
pub type History = fn(&mut Ctx<Parlor>) -> Vec<Track>;

static HISTORY: OnceLock<History> = OnceLock::new();

/// Lists the player's own songs, from their records. The launcher calls it
/// once; later calls are ignored.
pub fn register_history(history: History) {
    let _ = HISTORY.set(history);
}

/// Gives Hearken every track again, the player's own songs included, as
/// they stand in the records now.
pub fn refresh(ctx: &mut Ctx<Parlor>) {
    let mut all = tracks();
    if let Some(history) = HISTORY.get() {
        all.extend(history(ctx));
    }
    ctx.soundtrack.set_tracks(all);
    // Every album can be saved to sound files once all of it is heard,
    // each track as the player hears it in the game. An album with nothing
    // listed yet, such as a holiday special's before its season, offers no
    // export row, so Every album never names it.
    for album in ctx.soundtrack.albums() {
        let export = !ctx.soundtrack.listed(Some(&album)).is_empty();
        ctx.soundtrack.set_album(
            album,
            hearken::soundtrack::AlbumOptions {
                export,
                vary_export: false,
            },
        );
    }
}

/// How many of a game's own songs are kept, newest first.
pub const KEPT: usize = 30;

/// Keeps `entry` first in the list saved under `key`, dropping the oldest
/// past [`KEPT`].
pub fn keep<T: Serialize + DeserializeOwned>(ctx: &mut Ctx<Parlor>, key: &str, entry: T) {
    let mut all: Vec<T> = Records::load(ctx, key);
    all.insert(0, entry);
    all.truncate(KEPT);
    Records::store(ctx, key, &all);
}

/// The list saved under `key`, newest first.
pub fn kept<T: DeserializeOwned>(ctx: &mut Ctx<Parlor>, key: &str) -> Vec<T> {
    let all: Vec<T> = Records::load(ctx, key);
    all.into_iter().take(KEPT).collect()
}

/// "28 October 2026", for a history track's title.
pub fn date_words(d: Date) -> String {
    format!(
        "{} {} {}",
        d.day,
        crate::almanac::month_name(d.month),
        d.year
    )
}

/// Lists every track. The launcher calls it once, before the app starts;
/// later calls are ignored.
pub fn register(tracks: fn() -> Vec<Track>) {
    let _ = TRACKS.set(tracks);
}

/// Every registered track, or none before [`register`].
pub fn tracks() -> Vec<Track> {
    TRACKS.get().map(|f| f()).unwrap_or_default()
}

/// Stops hearing music as unlocking it, while a screen plays sounds out of
/// play, such as a Sound check. [`listen`] turns it back on.
pub fn quiet(ctx: &mut Ctx<Parlor>) {
    ctx.soundtrack.listen(false);
}

/// Hearing a track's music in play unlocks it again.
pub fn listen(ctx: &mut Ctx<Parlor>) {
    ctx.soundtrack.listen(true);
}

/// Unlocks the track of the music playing now, if it has one: for music
/// started before Hearken has begun listening, as the launcher's first
/// piece is.
pub fn heard_music(ctx: &mut Ctx<Parlor>) {
    if !ctx.soundtrack.is_listening() {
        return;
    }
    let Some(key) = ctx.audio.music().map(str::to_string) else {
        return;
    };
    let ids: Vec<String> = ctx
        .soundtrack
        .tracks()
        .iter()
        .filter(|t| t.unlocked_by.contains(&key))
        .map(|t| t.id.clone())
        .collect();
    for id in ids {
        ctx.unlock_track(&id);
    }
}

/// How long a short loop plays from the menu, about: enough times through
/// to sit back and hear it.
const SHORT_LOOP_S: f64 = 45.0;

/// Times through for a loop of `length_s` seconds.
pub fn repeats_for(length_s: f64) -> u32 {
    if length_s <= 0.0 {
        return 1;
    }
    (SHORT_LOOP_S / length_s).ceil().clamp(1.0, 6.0) as u32
}

/// One game's tracks, all under its name.
#[derive(Clone, Copy, Debug)]
pub struct Album {
    /// The game's id: its sounds are `<id>/...`.
    pub id: &'static str,
    /// The game's name, which the Soundtrack menu lists the album by.
    pub name: &'static str,
}

impl Album {
    pub const fn new(id: &'static str, name: &'static str) -> Album {
        Album { id, name }
    }

    /// The sound key `<id>/<path>`.
    pub fn key(&self, path: &str) -> String {
        format!("{}/{}", self.id, path)
    }

    /// A recorded piece at `<id>/<path>`, unlocked the first time it plays.
    /// `length_s` is the file's length: a loop shorter than about 45
    /// seconds plays through more than once from the menu.
    pub fn piece(&self, path: &str, title: &str, hint: &str, length_s: f64) -> Track {
        let key = self.key(path);
        Track::clip(key.clone(), title, key)
            .album(self.name)
            .hint(hint)
            .repeats(repeats_for(length_s))
    }

    /// The title screen's theme, at `music/title`.
    pub fn title(&self, length_s: f64) -> Track {
        self.title_at("music/title", length_s)
    }

    /// The title screen's theme, at `path`.
    pub fn title_at(&self, path: &str, length_s: f64) -> Track {
        self.piece(
            path,
            "Title theme",
            &format!("Open {}.", self.name),
            length_s,
        )
        .about(format!("The music on {}'s title.", self.name))
    }

    /// The pause menu's music, at `music/pause`.
    pub fn pause(&self, length_s: f64) -> Track {
        self.piece(
            "music/pause",
            "Pause theme",
            &format!("Pause a game of {}.", self.name),
            length_s,
        )
        .about(format!("The music while {} is paused.", self.name))
    }

    /// The game's dynamic score (see [`crate::score`]): its base, with
    /// the pulse, melody and lift coming and going, arranged anew each play.
    /// `loop_s` is the stems' length. Unlocked the first time the score
    /// starts in play.
    pub fn score(&self, loop_s: f64, hint: &str) -> Track {
        let base = self.key("score/base");
        Track::stems(
            format!("{}/score", self.id),
            "In play",
            Stems::new(loop_s).bed([base.clone()]).voices([
                self.key("score/pulse"),
                self.key("score/melody"),
                self.key("score/lift"),
            ]),
        )
        .unlocked_by([base])
        .album(self.name)
        .hint(hint)
        .about(
            "The music that follows play, its voices joining and leaving in a new order each time.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_loops_repeat_and_long_pieces_play_once() {
        assert_eq!(repeats_for(9.2), 5);
        assert_eq!(repeats_for(23.0), 2);
        assert_eq!(repeats_for(64.0), 1);
        assert_eq!(repeats_for(1.0), 6);
        assert_eq!(repeats_for(0.0), 1);
    }

    #[test]
    fn an_album_names_its_tracks_by_their_keys() {
        let a = Album::new("belfry", "Belfry");
        let t = a.title(26.7);
        assert_eq!(t.id, "belfry/music/title");
        assert_eq!(t.album, "Belfry");
        assert_eq!(t.unlocked_by, ["belfry/music/title"]);
        assert_eq!(t.hint, "Open Belfry.");
        let s = a.score(36.9, "Play a round.");
        assert_eq!(s.id, "belfry/score");
        assert_eq!(s.unlocked_by, ["belfry/score/base"]);
    }
}
