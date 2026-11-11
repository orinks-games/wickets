//! Your songs: the music a run made as it went, kept to hear again.
//!
//! A game whose music is built in play (voices joining as a garden grows,
//! a score swelling with the action) plays something different every run,
//! and no seed brings a run back once the music followed the player. So a
//! run is kept as a *take*: Hearken records every call the run makes on
//! the music bus ([`hearken::Take`]), and the Soundtrack plays those calls
//! again or writes them to a sound file. One file per run under
//! `songs/<game>/<n>.json` in the data folder, an index of them in the
//! records, [`KEPT`] newest per game, listed in an album of their own,
//! "Your Rooftop Garden songs", open from the start.
//!
//! A game calls [`begin`] as its run screen arrives and [`end`] as the run
//! ends (a game with a dynamic [`crate::Score`] gets both from the score).
//! Hearken holds the take while a pause or a menu covers the run, so a
//! song has no gap and no menu music. The shared results menu offers
//! "Make a sound file of this run's song" when the run just kept one. A
//! player who wants none turns "Keep your songs" off in Settings, under
//! Sound.

use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use hearken::screens::settings::categories;
use hearken::screens::SettingsRow;
use hearken::{Bus, Ctx, SoundtrackScreen, Take, TakeState, Track};
use serde::{Deserialize, Serialize};

use crate::{Date, Parlor, Records};

/// How many of a game's songs are kept, newest first.
pub const KEPT: usize = crate::soundtrack::KEPT;

/// A run shorter than this keeps no song: a round left at once, or a
/// title heard for a moment.
pub const MIN_S: f64 = 15.0;

/// The folder under the data folder that holds the song files.
pub const DIR: &str = "songs";

/// Where the player's choices about songs live in the records.
pub const PREFS_KEY: &str = "parlor/songs";

/// Said once, the first time a game without a results menu keeps a song.
pub const TOLD_LINE: &str = "Your song is kept in the Soundtrack.";

/// The results row that saves the run's song.
pub const SAVE_ROW: &str = "Make a sound file of this run's song";

/// The Settings row's id.
pub const ROW_ID: &str = "songs";

/// What the player chose about songs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prefs {
    /// Whether runs keep their songs. On until turned off.
    #[serde(default = "yes")]
    pub keep: bool,
    /// Whether [`TOLD_LINE`] has been said.
    #[serde(default)]
    pub told: bool,
}

fn yes() -> bool {
    true
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            keep: true,
            told: false,
        }
    }
}

/// Whether runs keep their songs.
pub fn keeping(ctx: &Ctx<Parlor>) -> bool {
    Records::peek::<Prefs>(ctx, PREFS_KEY).keep
}

pub fn set_keeping(ctx: &mut Ctx<Parlor>, on: bool) {
    let mut prefs: Prefs = Records::load(ctx, PREFS_KEY);
    prefs.keep = on;
    Records::store(ctx, PREFS_KEY, &prefs);
}

/// One kept song, as the records index it. The take itself is in its
/// file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// Counts up across every song a game kept, so each has its own file
    /// and track.
    pub n: u32,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    /// The hour the run ended, 0 to 23.
    pub hour: u32,
    /// How long the song runs.
    pub length_s: f64,
    /// Whether the take was cut at Hearken's limit.
    #[serde(default)]
    pub cut: bool,
}

impl Entry {
    fn date(&self) -> Date {
        Date::new(self.year, self.month, self.day)
    }

    /// "Evening, 9 November 2026".
    pub fn title(&self) -> String {
        format!(
            "{}, {}",
            part_of_day(self.hour),
            crate::soundtrack::date_words(self.date())
        )
    }

    /// "Your Rooftop Garden run on the evening of 9 November 2026, about
    /// twelve minutes, as you heard it."
    pub fn about(&self, game_name: &str) -> String {
        let when = part_of_day(self.hour).to_lowercase();
        let cut = if self.cut { " The first hour of it." } else { "" };
        format!(
            "Your {game_name} run on the {when} of {}, {}, as you heard it.{cut}",
            crate::soundtrack::date_words(self.date()),
            length_words(self.length_s)
        )
    }
}

/// "Morning", "Afternoon", "Evening" or "Night", for an hour of the day.
pub fn part_of_day(hour: u32) -> &'static str {
    match hour {
        5..=11 => "Morning",
        12..=16 => "Afternoon",
        17..=20 => "Evening",
        _ => "Night",
    }
}

/// How long a song runs, in words: "under a minute", "about a minute",
/// "about twelve minutes", "about an hour".
pub fn length_words(seconds: f64) -> String {
    let minutes = (seconds / 60.0).round() as u32;
    match minutes {
        0 => "under a minute".to_string(),
        1 => "about a minute".to_string(),
        2..=59 => format!("about {} minutes", minutes_words(minutes)),
        _ => "about an hour".to_string(),
    }
}

/// 2 to 59 in words: "two", "twenty-two".
fn minutes_words(n: u32) -> String {
    const ONES: [&str; 20] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 6] = ["", "", "twenty", "thirty", "forty", "fifty"];
    match n {
        0..=19 => ONES[n as usize].to_string(),
        20..=59 if n.is_multiple_of(10) => TENS[(n / 10) as usize].to_string(),
        20..=59 => format!("{}-{}", TENS[(n / 10) as usize], ONES[(n % 10) as usize]),
        _ => n.to_string(),
    }
}

/// A song a run just kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kept {
    /// The game's id.
    pub game: String,
    pub n: u32,
}

impl Kept {
    /// The song's track id on the soundtrack.
    pub fn track_id(&self) -> String {
        track_id(&self.game, self.n)
    }
}

/// The songs this session has loaded or kept, so a take is read from its
/// file once.
#[derive(Default)]
pub struct Songs {
    loaded: HashMap<(String, u32), Rc<Take>>,
    /// The song kept most recently this session, for the results menu.
    fresh: Option<Kept>,
}

/// The track id of a game's song `n`.
pub fn track_id(game: &str, n: u32) -> String {
    format!("{game}/song/{n}")
}

/// The album a game's songs are listed under.
pub fn album(game_name: &str) -> String {
    format!("Your {game_name} songs")
}

fn index_key(game: &str) -> String {
    format!("{game}/songs")
}

/// The folder a game's song files are in.
pub fn dir(ctx: &Ctx<Parlor>, game: &str) -> PathBuf {
    ctx.paths.data_dir.join(DIR).join(game)
}

fn file(ctx: &Ctx<Parlor>, game: &str, n: u32) -> PathBuf {
    dir(ctx, game).join(format!("{n}.json"))
}

/// Starts keeping the run's music, if the player keeps songs and nothing
/// is recording yet. The take belongs to the screen on top: call it as
/// the run screen arrives. A new run also puts the last run's song out
/// of reach of the results menu, so it never offers an older song as
/// this run's.
pub fn begin(ctx: &mut Ctx<Parlor>) {
    ctx.game.songs.fresh = None;
    if keeping(ctx) && ctx.take_state() == TakeState::Idle {
        ctx.begin_take(&[Bus::MUSIC]);
    }
}

/// Ends the run's take and keeps it as `game`'s newest song, dropping the
/// oldest past [`KEPT`]. `None` when nothing was recording, the run was
/// too short to keep, or the file could not be written. Calling it again
/// after the take ended is nothing, so a run may end its song both where
/// it finishes and as its screen leaves.
pub fn end(ctx: &mut Ctx<Parlor>, game: &str) -> Option<Kept> {
    let take = ctx.end_take()?;
    if take.starts() == 0 || take.length_s < MIN_S {
        log::info!(target: "wickets::songs", "{game}: run too short to keep ({:.0} s)", take.length_s);
        return None;
    }
    let now = crate::story::now(ctx);
    let key = index_key(game);
    let mut all: Vec<Entry> = Records::load(ctx, &key);
    let n = all.first().map_or(0, |e| e.n) + 1;
    let path = file(ctx, game, n);
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            log::warn!("songs: could not make {}: {e}", parent.display());
            return None;
        }
    }
    if let Err(e) = std::fs::write(&path, take.to_json()) {
        log::warn!("songs: could not write {}: {e}", path.display());
        return None;
    }
    all.insert(
        0,
        Entry {
            n,
            year: now.year,
            month: now.month,
            day: now.day,
            hour: now.hour,
            length_s: take.length_s,
            cut: take.cut,
        },
    );
    for old in all.drain(KEPT.min(all.len())..) {
        let gone = file(ctx, game, old.n);
        if let Err(e) = std::fs::remove_file(&gone) {
            log::warn!("songs: could not drop {}: {e}", gone.display());
        }
        ctx.game.songs.loaded.remove(&(game.to_string(), old.n));
    }
    Records::store(ctx, &key, &all);
    log::info!(target: "wickets::songs", "{game}: kept song {n}, {:.0} s, {} events", take.length_s, take.events.len());
    ctx.game
        .songs
        .loaded
        .insert((game.to_string(), n), Rc::new(take));
    let kept = Kept {
        game: game.to_string(),
        n,
    };
    ctx.game.songs.fresh = Some(kept.clone());
    Some(kept)
}

/// [`end`], and the first time a song is kept this way, [`TOLD_LINE`]
/// after what the game is saying: for a game whose run ends without the
/// shared results menu and its "Make a sound file of this run's song"
/// row.
pub fn end_and_tell(ctx: &mut Ctx<Parlor>, game: &str) -> Option<Kept> {
    let kept = end(ctx, game)?;
    let mut prefs: Prefs = Records::load(ctx, PREFS_KEY);
    if !prefs.told {
        prefs.told = true;
        Records::store(ctx, PREFS_KEY, &prefs);
        ctx.say_queued(TOLD_LINE);
    }
    Some(kept)
}

/// The song kept most recently this session, when it is `screen`'s
/// game's: screen names start with the game's id.
pub fn fresh_for(ctx: &Ctx<Parlor>, screen: &str) -> Option<Kept> {
    let kept = ctx.game.songs.fresh.clone()?;
    let screen = screen.replace('_', "-");
    screen.starts_with(&kept.game).then_some(kept)
}

/// The song kept most recently this session, for tests.
pub fn fresh(ctx: &Ctx<Parlor>) -> Option<Kept> {
    ctx.game.songs.fresh.clone()
}

/// Opens the Soundtrack on `kept` and starts saving it as a sound file.
pub fn save(ctx: &mut Ctx<Parlor>, kept: &Kept) {
    crate::soundtrack::refresh(ctx);
    let name = game_name(&kept.game);
    ctx.push(
        SoundtrackScreen::new()
            .album(album(&name))
            .saving(kept.track_id()),
    );
}

/// A game's name, from the launcher's roster; its id when the roster does
/// not know it.
fn game_name(game: &str) -> String {
    crate::preferences::games()
        .into_iter()
        .find(|g| g.id == game)
        .map_or_else(|| game.to_string(), |g| g.name.to_string())
}

/// The take of a game's song `n`, read from its file once.
fn load(ctx: &mut Ctx<Parlor>, game: &str, n: u32) -> Option<Rc<Take>> {
    let key = (game.to_string(), n);
    if let Some(take) = ctx.game.songs.loaded.get(&key) {
        return Some(take.clone());
    }
    let path = file(ctx, game, n);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| log::warn!("songs: could not read {}: {e}", path.display()))
        .ok()?;
    let take = Take::from_json(&text)
        .map_err(|e| log::warn!("songs: {} does not read: {e}", path.display()))
        .ok()?;
    let take = Rc::new(take);
    ctx.game.songs.loaded.insert(key, take.clone());
    Some(take)
}

/// A game's kept songs, newest first.
pub fn entries(ctx: &mut Ctx<Parlor>, game: &str) -> Vec<Entry> {
    Records::load(ctx, &index_key(game))
}

/// Every game's songs as tracks, newest first within each game's album,
/// for the soundtrack. A song whose file is gone or unreadable is left
/// out.
pub fn tracks(ctx: &mut Ctx<Parlor>) -> Vec<Track> {
    let games: Vec<(&str, &str)> = crate::preferences::games()
        .into_iter()
        .map(|g| (g.id, g.name))
        .collect();
    tracks_for(ctx, &games)
}

/// [`tracks`] for the given games, as `(id, name)`.
pub fn tracks_for(ctx: &mut Ctx<Parlor>, games: &[(&str, &str)]) -> Vec<Track> {
    let mut out = Vec::new();
    for &(id, name) in games {
        for entry in entries(ctx, id) {
            let Some(take) = load(ctx, id, entry.n) else {
                continue;
            };
            out.push(
                Track::take_shared(track_id(id, entry.n), entry.title(), take)
                    .album(album(name))
                    .about(entry.about(name)),
            );
        }
    }
    out
}

/// The Settings row, under Sound: "Keep your songs: on".
pub fn row() -> SettingsRow<Parlor> {
    crate::menus::settings::toggle(
        ROW_ID,
        |on| format!("Keep your songs: {}", if on { "on" } else { "off" }),
        keeping,
        set_keeping,
        |_ctx, on| {
            Some(if on {
                "Keep your songs: on. Each run's music is kept in the Soundtrack.".to_string()
            } else {
                "Keep your songs: off. Songs already kept stay.".to_string()
            })
        },
        "Keeps the music of each run you play as a song in the Soundtrack, to hear again or \
         save as a sound file. Enter, Left or Right changes it.",
    )
    .in_category(categories::SOUND)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::testing::Base;
    use hearken::{Headless, LocalTime, State};

    struct Run;
    impl State<Parlor> for Run {
        fn name(&self) -> &'static str {
            "tide-pools_run"
        }
        fn help(&self, _ctx: &Ctx<Parlor>) -> String {
            String::new()
        }
    }

    fn open() -> Headless<Parlor> {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.game.now = Some(LocalTime {
            year: 2026,
            month: 11,
            day: 9,
            hour: 19,
            minute: 30,
            utc_offset_minutes: 0,
        });
        h.app.ctx.push(Run);
        h.frame(0.0);
        h
    }

    /// A run of `seconds` with one loop on the music bus.
    fn run(h: &mut Headless<Parlor>, seconds: f64) {
        begin(&mut h.app.ctx);
        let s = h.app.ctx.audio.start_loop("tide-pools/score/base", None, Bus::MUSIC);
        h.frame(seconds);
        h.app.ctx.audio.stop(s, 300);
    }

    #[test]
    fn a_run_keeps_its_song_as_a_file_and_a_track() {
        let mut h = open();
        run(&mut h, 25.0);
        let kept = end(&mut h.app.ctx, "tide-pools").expect("kept");
        assert_eq!(kept, Kept { game: "tide-pools".into(), n: 1 });
        assert!(end(&mut h.app.ctx, "tide-pools").is_none(), "ended twice");
        let path = file(&h.app.ctx, "tide-pools", 1);
        assert!(path.is_file(), "{}", path.display());
        let entries = entries(&mut h.app.ctx, "tide-pools");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title(), "Evening, 9 November 2026");
        assert_eq!(
            entries[0].about("Tide Pools"),
            "Your Tide Pools run on the evening of 9 November 2026, under a minute, as you heard it."
        );
        assert_eq!(fresh_for(&h.app.ctx, "tide-pools_results"), Some(kept.clone()));
        assert_eq!(fresh_for(&h.app.ctx, "tightrope_results"), None);
        // The same take comes back as a track, read from its file once.
        h.app.ctx.game.songs.loaded.clear();
        let tracks = tracks_for(&mut h.app.ctx, &[("tide-pools", "Tide Pools")]);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].id, "tide-pools/song/1");
        assert_eq!(tracks[0].album, "Your Tide Pools songs");
        assert!(tracks[0].open);
        assert!(h.app.ctx.game.songs.loaded.contains_key(&("tide-pools".to_string(), 1)));
    }

    #[test]
    fn a_short_run_and_a_player_who_keeps_none_leave_nothing() {
        let mut h = open();
        run(&mut h, 5.0);
        assert!(end(&mut h.app.ctx, "tide-pools").is_none());
        assert!(entries(&mut h.app.ctx, "tide-pools").is_empty());
        set_keeping(&mut h.app.ctx, false);
        assert!(!keeping(&h.app.ctx));
        begin(&mut h.app.ctx);
        assert_eq!(h.app.ctx.take_state(), TakeState::Idle);
        assert!(end(&mut h.app.ctx, "tide-pools").is_none());
    }

    #[test]
    fn a_new_run_forgets_the_last_runs_song() {
        // The results menu offers only this run's song: once another run
        // begins, the last one is in the Soundtrack and nowhere else.
        let mut h = open();
        run(&mut h, 25.0);
        assert!(end(&mut h.app.ctx, "tide-pools").is_some());
        assert!(fresh_for(&h.app.ctx, "tide-pools_run").is_some());
        set_keeping(&mut h.app.ctx, false);
        begin(&mut h.app.ctx);
        assert!(fresh_for(&h.app.ctx, "tide-pools_run").is_none());
        assert!(end(&mut h.app.ctx, "tide-pools").is_none());
        assert_eq!(entries(&mut h.app.ctx, "tide-pools").len(), 1);
    }

    #[test]
    fn only_the_newest_thirty_are_kept_and_old_files_go() {
        let mut h = open();
        for _ in 0..(KEPT + 2) {
            run(&mut h, 20.0);
            end(&mut h.app.ctx, "tide-pools").expect("kept");
        }
        let entries = entries(&mut h.app.ctx, "tide-pools");
        assert_eq!(entries.len(), KEPT);
        assert_eq!(entries[0].n, (KEPT + 2) as u32);
        assert!(!file(&h.app.ctx, "tide-pools", 1).exists());
        assert!(!file(&h.app.ctx, "tide-pools", 2).exists());
        assert!(file(&h.app.ctx, "tide-pools", 3).exists());
    }

    #[test]
    fn the_first_song_kept_without_results_is_told_once() {
        let mut h = open();
        run(&mut h, 20.0);
        h.clear();
        end_and_tell(&mut h.app.ctx, "tide-pools").expect("kept");
        h.frame(0.0);
        assert_eq!(h.spoken(), vec![TOLD_LINE]);
        run(&mut h, 20.0);
        h.clear();
        end_and_tell(&mut h.app.ctx, "tide-pools").expect("kept");
        h.frame(0.0);
        assert!(h.spoken().is_empty(), "{:?}", h.spoken());
    }

    #[test]
    fn words_for_the_time_of_day_and_the_length() {
        assert_eq!(part_of_day(6), "Morning");
        assert_eq!(part_of_day(13), "Afternoon");
        assert_eq!(part_of_day(19), "Evening");
        assert_eq!(part_of_day(23), "Night");
        assert_eq!(part_of_day(2), "Night");
        assert_eq!(length_words(20.0), "under a minute");
        assert_eq!(length_words(70.0), "about a minute");
        assert_eq!(length_words(12.0 * 60.0), "about twelve minutes");
        assert_eq!(length_words(22.0 * 60.0), "about twenty-two minutes");
        assert_eq!(length_words(40.0 * 60.0), "about forty minutes");
        assert_eq!(length_words(3600.0), "about an hour");
    }
}
