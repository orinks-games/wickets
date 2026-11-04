//! Every game's saved progress, in one `records.json` in the data folder.
//!
//! A game keeps whatever it likes under its own id: best scores through the
//! helpers here, anything else as JSON through [`Records::load`] and
//! [`Records::store`].
//!
//! The file goes through Hearken's vault (`ctx.vault`). In a release built
//! with `seal = true` it is sealed, so a player who edits or forges it is
//! caught: the records start over and the changed file is set aside.
//! Putting back an older copy, or deleting it, is not caught; trusted
//! results need the server. Developer builds and tests read and write plain JSON as
//! before.
//!
//! Players who had plain records from before sealing keep them: the first
//! sealing release reads a plain `records.json` once ([`IMPORT_LEGACY`]),
//! seals it at once, and the vault keeps the plain copy as
//! `records.json.plain-backup`. Hearken only offers that import while none
//! of the markers the vault leaves outside the folder (in the config
//! folder, or beside a portable copy's executable) is there, so deleting
//! the records, or switching to a fresh folder, does not reopen it. The cost that
//! remains: a player who also deletes those markers, or plays as a new
//! user, can hand the next start a plain file of their own, and then copy
//! the sealed folder that import makes over their real data folder. Turn
//! [`IMPORT_LEGACY`] (`IMPORTS_PLAIN_SAVES` in `lib.rs`) off once players
//! have had a sealing release long
//! enough to move over; from then on no plain file is ever sealed, so
//! that route closes.

use std::collections::BTreeMap;
use std::path::PathBuf;

use hearken::{Ctx, Opened, Vault};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::Parlor;

/// The records' file name in the data folder.
pub const FILE: &str = "records.json";

/// Whether a sealing build imports a plain `records.json` it finds with no
/// sealed history, as an older version of the collection left it. Set on
/// `Parlor`'s `Game` impl, which also tells Hearken to leave its marker.
pub const IMPORT_LEGACY: bool = <Parlor as hearken::Game>::IMPORTS_PLAIN_SAVES;

/// What the player hears when their records were changed outside the game.
pub const TAMPERED_NOTICE: &str = "Your records were changed outside the game, so they start over.";

/// What the player hears when the records file is damaged.
pub const BROKEN_NOTICE: &str =
    "Your records were damaged, so they start over. The old file is kept.";

/// What the player hears when the records could not be read.
pub const UNREADABLE_NOTICE: &str =
    "Your records could not be read, so nothing is saved this time.";

#[derive(Default, Serialize, Deserialize)]
struct File {
    #[serde(default)]
    games: BTreeMap<String, serde_json::Value>,
}

/// What recording a score did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScoreResult {
    /// The first score saved for this board.
    First,
    /// Beat the old best, which is given.
    NewBest { previous: u64 },
    /// Did not beat the best, which is given.
    Short { best: u64 },
}

/// The saved records, loaded from disk on first use.
#[derive(Default)]
pub struct Records {
    loaded: Option<Loaded>,
    /// Something the player should hear about the records, once.
    notice: Option<&'static str>,
}

/// The records as read from one data folder.
struct Loaded {
    dir: PathBuf,
    file: File,
    /// False when saving could lose something: a file this build cannot
    /// open, or one the disk would not give up.
    writable: bool,
}

impl Records {
    fn file<'a>(&'a mut self, vault: &mut Vault) -> &'a mut File {
        let stale = !matches!(&self.loaded, Some(l) if l.dir == vault.dir());
        if stale {
            self.loaded = Some(self.open(vault));
        }
        &mut self.loaded.as_mut().expect("just loaded").file
    }

    /// Reads the records through the vault and decides what to trust.
    fn open(&mut self, vault: &mut Vault) -> Loaded {
        let dir = vault.dir().to_path_buf();
        let fresh = |writable| Loaded {
            dir: dir.clone(),
            file: File::default(),
            writable,
        };
        let parse = |bytes: &[u8]| match serde_json::from_slice::<File>(bytes) {
            Ok(file) => Some(file),
            Err(e) => {
                log::warn!("{FILE} is unreadable, starting fresh: {e}");
                None
            }
        };
        match vault.read(FILE) {
            Opened::Missing => fresh(true),
            Opened::Sealed(bytes) | Opened::Plain(bytes) => match parse(&bytes) {
                Some(file) => Loaded {
                    file,
                    ..fresh(true)
                },
                None => {
                    // Keep the broken file, so the first save cannot
                    // destroy the only copy.
                    let kept = self.set_aside(vault);
                    self.notice = Some(BROKEN_NOTICE);
                    fresh(kept)
                }
            },
            Opened::Legacy(bytes) if IMPORT_LEGACY => {
                log::info!("importing plain {FILE} from before sealing");
                let loaded = Loaded {
                    file: parse(&bytes).unwrap_or_default(),
                    ..fresh(true)
                };
                // Seal it now, so the import happens once; the vault keeps
                // the plain file as a backup.
                write(vault, &loaded.file);
                loaded
            }
            Opened::Legacy(_) => {
                log::warn!("a plain {FILE} is not imported; starting fresh");
                self.set_aside(vault);
                fresh(true)
            }
            Opened::Tampered(how) => {
                log::warn!("{FILE} was changed outside the game ({how}); starting fresh");
                self.set_aside(vault);
                self.notice = Some(TAMPERED_NOTICE);
                fresh(true)
            }
            Opened::Locked => {
                log::warn!(
                    "{FILE} is sealed by a release and this build cannot open it; \
                     records will not be saved"
                );
                fresh(false)
            }
            Opened::Failed(e) => {
                log::warn!("could not read {FILE}, records will not be saved: {e}");
                self.notice = Some(UNREADABLE_NOTICE);
                fresh(false)
            }
        }
    }

    /// Moves a file the game will not trust out of the way, keeping it.
    /// False when it could not be moved.
    fn set_aside(&mut self, vault: &mut Vault) -> bool {
        match vault.set_aside(FILE) {
            Ok(Some(to)) => {
                log::warn!("kept the old {FILE} as {}", to.display());
                true
            }
            Ok(None) => true,
            Err(e) => {
                log::warn!("could not set {FILE} aside: {e}");
                false
            }
        }
    }

    fn save(&mut self, vault: &mut Vault) {
        self.file(vault);
        let loaded = self.loaded.as_ref().expect("just loaded");
        if loaded.writable {
            write(vault, &loaded.file);
        }
    }

    /// What the player should hear about their records, once: that they
    /// were changed outside the game and start over, or could not be read.
    /// Reads the records first if nothing has yet.
    pub fn take_notice(ctx: &mut Ctx<Parlor>) -> Option<&'static str> {
        let records = &mut ctx.game.records;
        records.file(&mut ctx.vault);
        records.notice.take()
    }

    /// A game's saved value from records already read, without the disk,
    /// for code that only has `&Ctx` such as a Settings row. The default
    /// before anything has loaded the records.
    pub fn peek<T: DeserializeOwned + Default>(ctx: &Ctx<Parlor>, game: &str) -> T {
        ctx.game
            .records
            .loaded
            .as_ref()
            .and_then(|l| l.file.games.get(game))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    /// A game's saved value, or its default when none is saved or it no
    /// longer reads.
    pub fn load<T: DeserializeOwned + Default>(ctx: &mut Ctx<Parlor>, game: &str) -> T {
        ctx.game
            .records
            .file(&mut ctx.vault)
            .games
            .get(game)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    /// Saves a game's value, to disk at once.
    pub fn store<T: Serialize>(ctx: &mut Ctx<Parlor>, game: &str, value: &T) {
        let Ok(value) = serde_json::to_value(value) else {
            log::warn!("could not encode {game}'s record");
            return;
        };
        let records = &mut ctx.game.records;
        records
            .file(&mut ctx.vault)
            .games
            .insert(game.to_string(), value);
        records.save(&mut ctx.vault);
    }

    /// The best score on one of a game's boards (`"best"`, `"freestyle"`,
    /// `"level 3"`...).
    pub fn best(ctx: &mut Ctx<Parlor>, game: &str, board: &str) -> Option<u64> {
        let scores: BTreeMap<String, u64> = Self::load(ctx, &format!("{game}/scores"));
        scores.get(board).copied()
    }

    /// Records a score, keeping the higher of it and the old best.
    pub fn record(ctx: &mut Ctx<Parlor>, game: &str, board: &str, score: u64) -> ScoreResult {
        let key = format!("{game}/scores");
        let mut scores: BTreeMap<String, u64> = Self::load(ctx, &key);
        let result = match scores.get(board).copied() {
            None => ScoreResult::First,
            Some(previous) if score > previous => ScoreResult::NewBest { previous },
            Some(best) => ScoreResult::Short { best },
        };
        if !matches!(result, ScoreResult::Short { .. }) {
            scores.insert(board.to_string(), score);
            Self::store(ctx, &key, &scores);
        }
        crate::tutorial::mark_played(ctx, game.split('/').next().unwrap_or(game));
        result
    }
}

/// Writes the records through the vault, logging what goes wrong.
fn write(vault: &mut Vault, file: &File) {
    match serde_json::to_vec_pretty(file) {
        Ok(bytes) => {
            if let Err(e) = vault.write(FILE, &bytes) {
                log::warn!("could not save records: {e}");
            }
        }
        Err(e) => log::warn!("could not encode records: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use hearken::seal::Key;
    use hearken::{Headless, Vault};

    use super::*;
    use crate::menus::testing::Base;

    fn key() -> Key {
        Key::from_bytes([7; 32])
    }

    /// A headless parlor whose vault seals, as a release's does.
    fn sealing() -> Headless<Parlor> {
        let mut h = Headless::new(Default::default(), Base);
        let dir = h.app.ctx.paths.data_dir.clone();
        h.app.ctx.vault = Vault::with_key(dir, Some(key()));
        h
    }

    /// The same folder opened again, as the next start of the game.
    fn restart(h: &mut Headless<Parlor>) {
        let dir = h.app.ctx.paths.data_dir.clone();
        h.app.ctx.vault = Vault::with_key(dir, Some(key()));
        h.app.ctx.game.records = Records::default();
    }

    /// A sealing parlor with a witness, as a release's vault has.
    fn witnessed() -> Headless<Parlor> {
        let mut h = Headless::new(Default::default(), Base);
        reopen_witnessed(&mut h);
        h
    }

    /// The next start of a witnessed parlor, on whatever folder it now has.
    fn reopen_witnessed(h: &mut Headless<Parlor>) {
        let dir = h.app.ctx.paths.data_dir.clone();
        // Inside the test's own folder only so it is cleaned up; the attacks
        // below delete just the vault's own files, as a player would.
        let witness = dir.join("elsewhere").join("witness");
        h.app.ctx.vault = Vault::with_key(dir, Some(key())).with_witnesses(vec![witness]);
        h.app.ctx.game.records = Records::default();
    }

    fn path(h: &Headless<Parlor>) -> std::path::PathBuf {
        h.app.ctx.paths.data_dir.join(FILE)
    }

    fn mentions(path: &Path, text: &str) -> bool {
        std::fs::read(path)
            .map(|b| String::from_utf8_lossy(&b).contains(text))
            .unwrap_or(false)
    }

    #[test]
    fn a_developer_build_keeps_plain_json() {
        let mut h = Headless::new(Default::default(), Base);
        Records::record(&mut h.app.ctx, "dummy", "best", 12);
        assert!(mentions(&path(&h), "\"best\": 12"));
        h.app.ctx.game.records = Records::default();
        assert_eq!(Records::best(&mut h.app.ctx, "dummy", "best"), Some(12));
    }

    #[test]
    fn a_damaged_file_is_kept_aside_not_overwritten() {
        let mut h = Headless::new(Default::default(), Base);
        let broken = r#"{"games":{"dummy/scores":{"best":4"#;
        std::fs::write(path(&h), broken).unwrap();
        assert_eq!(Records::best(&mut h.app.ctx, "dummy", "best"), None);
        Records::record(&mut h.app.ctx, "dummy", "best", 7);
        let kept = h.app.ctx.paths.data_dir.join("records.json.tampered");
        assert_eq!(std::fs::read_to_string(kept).unwrap(), broken);
        assert_eq!(Records::take_notice(&mut h.app.ctx), Some(BROKEN_NOTICE));
        assert!(mentions(&path(&h), "\"best\": 7"));
    }

    #[test]
    fn a_sealing_build_saves_nothing_readable_and_reads_it_back() {
        let mut h = sealing();
        Records::record(&mut h.app.ctx, "dummy", "best", 12);
        assert!(path(&h).exists());
        assert!(!mentions(&path(&h), "dummy"));
        restart(&mut h);
        assert_eq!(Records::best(&mut h.app.ctx, "dummy", "best"), Some(12));
        assert_eq!(Records::take_notice(&mut h.app.ctx), None);
    }

    #[test]
    fn plain_records_from_before_sealing_are_imported_once() {
        let mut h = sealing();
        let plain = r#"{"games":{"dummy/scores":{"best":40}}}"#;
        std::fs::write(path(&h), plain).unwrap();
        assert_eq!(Records::best(&mut h.app.ctx, "dummy", "best"), Some(40));
        // Sealed at once, with the plain file kept beside it.
        assert!(!mentions(&path(&h), "dummy"));
        let backup = h.app.ctx.paths.data_dir.join("records.json.plain-backup");
        assert_eq!(std::fs::read_to_string(backup).unwrap(), plain);
        restart(&mut h);
        assert_eq!(Records::best(&mut h.app.ctx, "dummy", "best"), Some(40));
        assert_eq!(Records::take_notice(&mut h.app.ctx), None);
    }

    #[test]
    fn an_edited_file_starts_over_and_says_so() {
        let mut h = sealing();
        Records::record(&mut h.app.ctx, "dummy", "best", 12);
        let mut bytes = std::fs::read(path(&h)).unwrap();
        let last = bytes.len() - 20;
        bytes[last] ^= 1;
        std::fs::write(path(&h), bytes).unwrap();
        restart(&mut h);
        assert_eq!(Records::best(&mut h.app.ctx, "dummy", "best"), None);
        assert_eq!(Records::take_notice(&mut h.app.ctx), Some(TAMPERED_NOTICE));
        assert_eq!(Records::take_notice(&mut h.app.ctx), None);
        assert!(h
            .app
            .ctx
            .paths
            .data_dir
            .join("records.json.tampered")
            .exists());
        // Play goes on, and saves again.
        Records::record(&mut h.app.ctx, "dummy", "best", 3);
        restart(&mut h);
        assert_eq!(Records::best(&mut h.app.ctx, "dummy", "best"), Some(3));
        assert_eq!(Records::take_notice(&mut h.app.ctx), None);
    }

    #[test]
    fn a_hand_written_plain_file_over_sealed_records_is_refused() {
        let mut h = witnessed();
        Records::record(&mut h.app.ctx, "dummy", "best", 12);
        std::fs::write(path(&h), r#"{"games":{"dummy/scores":{"best":999}}}"#).unwrap();
        reopen_witnessed(&mut h);
        assert_eq!(Records::best(&mut h.app.ctx, "dummy", "best"), None);
        assert_eq!(Records::take_notice(&mut h.app.ctx), Some(TAMPERED_NOTICE));
    }

    #[test]
    fn a_developer_build_never_saves_over_a_release_s_records() {
        let mut h = sealing();
        Records::record(&mut h.app.ctx, "dummy", "best", 12);
        let sealed = std::fs::read(path(&h)).unwrap();
        let dir = h.app.ctx.paths.data_dir.clone();
        h.app.ctx.vault = Vault::with_key(dir, None);
        h.app.ctx.game.records = Records::default();
        assert_eq!(Records::best(&mut h.app.ctx, "dummy", "best"), None);
        Records::record(&mut h.app.ctx, "dummy", "best", 99);
        assert_eq!(std::fs::read(path(&h)).unwrap(), sealed);
        assert_eq!(Records::take_notice(&mut h.app.ctx), None);
    }

    const FORGED: &str = r#"{"games":{"dummy/scores":{"best":999999}}}"#;

    #[test]
    fn a_plain_file_after_tampered_records_were_set_aside_is_refused() {
        let mut h = witnessed();
        Records::record(&mut h.app.ctx, "dummy", "best", 20);
        std::fs::write(path(&h), b"cut short").unwrap();
        reopen_witnessed(&mut h);
        assert_eq!(Records::best(&mut h.app.ctx, "dummy", "best"), None);
        assert_eq!(Records::take_notice(&mut h.app.ctx), Some(TAMPERED_NOTICE));
        std::fs::write(path(&h), FORGED).unwrap();
        reopen_witnessed(&mut h);
        assert_eq!(Records::best(&mut h.app.ctx, "dummy", "best"), None);
        assert_eq!(Records::take_notice(&mut h.app.ctx), Some(TAMPERED_NOTICE));
    }

    #[test]
    fn deleting_the_records_does_not_reopen_the_import() {
        let mut h = witnessed();
        Records::record(&mut h.app.ctx, "dummy", "best", 20);
        std::fs::remove_file(path(&h)).unwrap();
        std::fs::write(path(&h), FORGED).unwrap();
        reopen_witnessed(&mut h);
        assert_eq!(Records::best(&mut h.app.ctx, "dummy", "best"), None);
        assert_eq!(Records::take_notice(&mut h.app.ctx), Some(TAMPERED_NOTICE));
    }

    #[test]
    fn a_fresh_folder_with_plain_records_is_refused_once_sealed_elsewhere() {
        let mut h = witnessed();
        Records::record(&mut h.app.ctx, "dummy", "best", 20);
        let witness = h.app.ctx.paths.data_dir.join("elsewhere").join("witness");
        assert!(witness.is_file());
        // As after removing portable.txt: another folder, the same player.
        let fresh = h.app.ctx.paths.data_dir.join("fresh");
        std::fs::create_dir(&fresh).unwrap();
        std::fs::write(fresh.join(FILE), FORGED).unwrap();
        h.app.ctx.vault = Vault::with_key(fresh, Some(key())).with_witnesses(vec![witness]);
        h.app.ctx.game.records = Records::default();
        assert_eq!(Records::best(&mut h.app.ctx, "dummy", "best"), None);
        assert_eq!(Records::take_notice(&mut h.app.ctx), Some(TAMPERED_NOTICE));
    }
}
