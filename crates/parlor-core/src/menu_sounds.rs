//! Each game's own menu sounds.
//!
//! A game can give its menus a voice of its own by shipping any of
//! `sounds/<id>/menu/move.ogg`, `select.ogg`, `back.ogg`, `open.ogg` and
//! `page.ogg`. While the game is open its title screen points Hearken's UI
//! earcons (and the page cue) at those keys; a key the game does not ship
//! keeps the shared sound (`ui/menu_*`, `parlor/page`). Back in the
//! launcher, the shared set returns.
//!
//! The same goes for the pause screen's music: a game that ships
//! `sounds/<id>/music/pause.ogg` hears it while paused, and any other hears
//! the shared `parlor/music/pause`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use hearken::{Ctx, UiSound, UiSounds};

use crate::pages::PAGE_SOUND;
use crate::Parlor;

/// The pause music for a game that ships none of its own.
pub const PAUSE_MUSIC: &str = "parlor/music/pause";

/// The earcons a game may replace, and the file name of each under
/// `sounds/<id>/menu/`.
pub const KINDS: [(UiSound, &str); 4] = [
    (UiSound::Move, "move"),
    (UiSound::Select, "select"),
    (UiSound::Back, "back"),
    (UiSound::Open, "open"),
];

/// The key of game `id`'s menu sound `name`, such as `belfry/menu/move`.
pub fn key(id: &str, name: &str) -> String {
    format!("{id}/menu/{name}")
}

/// `key` as a `&'static str`, which `UiSounds` needs. Each key is made once
/// and kept, so opening a game again costs nothing.
fn intern(key: String) -> &'static str {
    static KEYS: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let mut keys = KEYS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(k) = keys.get(&key) {
        return k;
    }
    let k: &'static str = Box::leak(key.clone().into_boxed_str());
    keys.insert(key, k);
    k
}

/// Points the menu earcons and the page cue at game `id`'s own sounds,
/// keeping the shared sound for any it does not ship.
pub fn use_game(ctx: &mut Ctx<Parlor>, id: &str) {
    let shared = UiSounds::default();
    for (kind, name) in KINDS {
        let own = intern(key(id, name));
        let pick = if ctx.audio.has_clip(own) {
            Some(own)
        } else {
            shared.get(kind)
        };
        ctx.ui_sounds.set(kind, pick);
    }
    ctx.game.open_game = Some(intern(id.to_string()));
    let page = intern(key(id, "page"));
    ctx.game.page_sound = ctx.audio.has_clip(page).then_some(page);
    let pause = intern(format!("{id}/music/pause"));
    ctx.game.pause_music = ctx.audio.has_clip(pause).then_some(pause);
    crate::units::sync(ctx);
}

/// Puts the launcher's shared menu sounds back.
pub fn use_shared(ctx: &mut Ctx<Parlor>) {
    let shared = UiSounds::default();
    for (kind, _) in KINDS {
        ctx.ui_sounds.set(kind, shared.get(kind));
    }
    ctx.game.open_game = None;
    ctx.game.open_game_name = None;
    ctx.game.page_sound = None;
    ctx.game.pause_music = None;
    ctx.game.game_settings = None;
    crate::units::sync(ctx);
}

/// The cue that starts each page: the open game's own, or the shared one.
pub fn page(ctx: &Ctx<Parlor>) -> &'static str {
    ctx.game.page_sound.unwrap_or(PAGE_SOUND)
}

/// The loop the pause screen plays: the open game's own, or the shared one.
pub fn pause_music(ctx: &Ctx<Parlor>) -> &'static str {
    ctx.game.pause_music.unwrap_or(PAUSE_MUSIC)
}
