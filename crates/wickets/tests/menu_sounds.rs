use std::path::Path;

use hearken::{Headless, Key, UiSound};
use parlor_core::menu_sounds::{self, KINDS};
use parlor_core::Parlor;
use wickets::{games, Lobby};

/// The menu sounds every game ships of its own; open and page may follow.
const OWN: [&str; 3] = ["move", "select", "back"];

#[test]
fn every_game_ships_its_own_menu_sounds() {
    let sounds = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds");
    let mut missing = Vec::new();
    for g in games() {
        for name in OWN {
            let file = sounds.join(format!("{}.ogg", menu_sounds::key(g.id, name)));
            if !file.exists() {
                missing.push(menu_sounds::key(g.id, name));
            }
        }
    }
    assert!(missing.is_empty(), "no menu sound at {missing:?}");
}

#[test]
fn every_game_ships_its_own_pause_music() {
    let sounds = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sounds");
    let mut missing: Vec<String> = games()
        .iter()
        .map(|g| format!("{}/music/pause", g.id))
        .filter(|key| !sounds.join(format!("{key}.ogg")).exists())
        .collect();
    let shared = menu_sounds::PAUSE_MUSIC;
    if !sounds.join(format!("{shared}.ogg")).exists() {
        missing.push(shared.to_string());
    }
    assert!(missing.is_empty(), "no pause music at {missing:?}");
}

fn shared(h: &Headless<Parlor>) -> bool {
    KINDS
        .iter()
        .all(|(kind, _)| h.app.ctx.ui_sounds.get(*kind) == hearken::UiSounds::default().get(*kind))
        && h.app.ctx.game.page_sound.is_none()
        && h.app.ctx.game.pause_music.is_none()
}

#[test]
fn a_game_without_its_own_sounds_keeps_the_shared_ones() {
    let mut h = Headless::new(Default::default(), Lobby::new());
    h.app
        .ctx
        .ui_sounds
        .set(UiSound::Move, Some("elsewhere/move"));
    // The test build has no sound files, so nothing of the game's is there.
    menu_sounds::use_game(&mut h.app.ctx, "liberty-lights");
    assert!(shared(&h));
}

#[test]
fn the_launcher_puts_the_shared_sounds_back() {
    // A day with no holiday, the Fourth of July next, so Liberty Lights
    // leads Seasonal games.
    let data = parlor_core::ParlorData {
        today: Some(parlor_core::Date::new(2027, 6, 24)),
        ..Default::default()
    };
    let mut h = Headless::new(data, Lobby::new());
    // Play, Seasonal games, Liberty Lights.
    h.press(Key::Return);
    h.press(Key::Return);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights");
    // As if the game had its own move sound.
    h.app
        .ctx
        .ui_sounds
        .set(UiSound::Move, Some("liberty-lights/menu/move"));
    h.app.ctx.game.page_sound = Some("liberty-lights/menu/page");
    h.app.ctx.game.pause_music = Some("liberty-lights/music/pause");
    h.press(Key::Escape);
    assert!(shared(&h));
}

#[test]
fn the_story_sitting_puts_the_shared_sounds_back() {
    let mut h = Headless::new(Default::default(), Lobby::new());
    // As if a story scene's game had left its own sounds in place.
    h.app
        .ctx
        .ui_sounds
        .set(UiSound::Move, Some("liberty-lights/menu/move"));
    h.app.ctx.game.page_sound = Some("liberty-lights/menu/page");
    h.app.ctx.push(wickets::story::SittingMenu::new());
    h.frame(0.0);
    assert_eq!(h.app.top_name(), "story");
    assert!(shared(&h));
}

#[test]
fn the_lobby_puts_the_shared_sounds_back() {
    // Continue opens a game straight from the lobby, and the pause menu's
    // Back to the lobby returns there without the game list in between.
    let mut h = Headless::new(Default::default(), Lobby::new());
    (games()[0].open)(&mut h.app.ctx);
    h.frame(0.0);
    h.app
        .ctx
        .ui_sounds
        .set(UiSound::Move, Some("liberty-lights/menu/move"));
    h.app.ctx.game.page_sound = Some("liberty-lights/menu/page");
    h.app.ctx.game.pause_music = Some("liberty-lights/music/pause");
    h.app.ctx.pop();
    h.frame(0.0);
    assert_eq!(h.app.top_name(), "lobby");
    assert!(shared(&h));
}
