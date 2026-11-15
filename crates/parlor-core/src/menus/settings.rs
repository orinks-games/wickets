//! A game's own rows on the shared Settings screen, such as spoken
//! call-outs. A game returns them from [`TitleGame::settings`]; they show
//! after the shared rows while the game is open, from its title or from the
//! pause menu during a run, and never on the title itself.
//!
//! [`TitleGame::settings`]: super::TitleGame::settings

use std::rc::Rc;

use hearken::menu::{Label, Menu, MenuItem};
use hearken::screens::{SettingsRow, SettingsScreen};
use hearken::Ctx;

use crate::Parlor;

/// One of a game's Settings rows. `label` reads the row with its value,
/// such as "Spoken call-outs: on", from records already read
/// (`Records::peek`). `change` steps the value by `delta`: Enter steps by 1,
/// Left and Right by -1 and 1. What it returns is said after Enter, in
/// place of the row, to explain the new value; `None` reads the row back.
/// Left and Right always read the row back.
pub fn setting(
    id: &str,
    label: impl Fn(&Ctx<Parlor>) -> String + 'static,
    change: impl Fn(&mut Ctx<Parlor>, i32) -> Option<String> + 'static,
    help: &str,
) -> SettingsRow<Parlor> {
    let change = Rc::new(change);
    let adjust = Rc::clone(&change);
    SettingsRow::new(
        id,
        MenuItem::new(
            Label::dynamic(move |_s: &SettingsScreen<Parlor>, ctx: &Ctx<Parlor>| label(ctx)),
            move |s: &mut SettingsScreen<Parlor>, ctx: &mut Ctx<Parlor>| {
                let said = change(ctx, 1);
                Menu::refresh(s, ctx, true);
                match said {
                    Some(text) => ctx.say(text),
                    None => s.speak_current(ctx),
                }
            },
        )
        .help(help)
        .adjustable(
            move |_s: &mut SettingsScreen<Parlor>, ctx: &mut Ctx<Parlor>, delta| {
                adjust(ctx, delta);
            },
        ),
    )
}

/// An on and off row, such as `labels::call_outs`: Enter, Left and Right
/// all flip it. `get` reads the value from records already read, `set`
/// saves it, and `said` explains the new value after Enter.
pub fn toggle(
    id: &str,
    label: fn(bool) -> String,
    get: fn(&Ctx<Parlor>) -> bool,
    set: fn(&mut Ctx<Parlor>, bool),
    said: fn(&Ctx<Parlor>, bool) -> Option<String>,
    help: &str,
) -> SettingsRow<Parlor> {
    setting(
        id,
        move |ctx| label(get(ctx)),
        move |ctx, _delta| {
            let on = !get(ctx);
            set(ctx, on);
            said(ctx, on)
        },
        help,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::labels;
    use crate::menus::testing::Base;
    use crate::Records;
    use crate::OpenGame;
    use hearken::{Headless, Key};

    const KEY: &str = "dummy/call_outs";

    fn get(ctx: &Ctx<Parlor>) -> bool {
        Records::peek(ctx, KEY)
    }
    fn set(ctx: &mut Ctx<Parlor>, on: bool) {
        Records::store(ctx, KEY, &on);
    }
    fn said(_ctx: &Ctx<Parlor>, on: bool) -> Option<String> {
        on.then(|| "Spoken call-outs: on. Each cue is named.".to_string())
    }
    fn rows(_ctx: &Ctx<Parlor>) -> Vec<SettingsRow<Parlor>> {
        vec![toggle(
            "dummy_call_outs",
            labels::call_outs,
            get,
            set,
            said,
            "Speak each cue.",
        )]
    }

    fn settings() -> Headless<Parlor> {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.game.open = Some(OpenGame::new("game", "Game", rows));
        h.app.ctx.push(SettingsScreen::<Parlor>::new());
        h.frame(0.0);
        h
    }

    /// The game's own category, open.
    fn game_rows() -> Headless<Parlor> {
        let mut h = settings();
        h.press(Key::Return);
        h.frame(0.0);
        h
    }

    #[test]
    fn the_games_rows_come_first_and_units_last() {
        let mut h = settings();
        assert!(
            h.spoken().iter().any(|l| l.starts_with("Game")),
            "Settings should open on the game's own category: {:?}",
            h.spoken()
        );
        assert_eq!(
            crate::menus::testing::rows(&h)[..5],
            ["Game", "Speech", "Sound", "Display", "Controls"]
        );
        h.clear();
        h.press(Key::Return);
        h.frame(0.0);
        assert!(
            h.spoken()
                .iter()
                .any(|l| l.starts_with("Spoken call-outs: off.")),
            "the category should open on the game's row: {:?}",
            h.spoken()
        );
        h.clear();
        h.press(Key::End);
        assert!(
            h.spoken()[0].starts_with("Units: metric"),
            "{:?}",
            h.spoken()
        );
        h.clear();
        h.press(Key::Return);
        assert_eq!(h.spoken(), ["Units: US. Feet, miles and miles an hour."]);
        assert!(crate::units::us());
        crate::units::set_current(crate::units::Units::Metric);
    }

    #[test]
    fn a_toggle_flips_saves_and_explains() {
        let mut h = game_rows();
        assert!(h
            .spoken()
            .last()
            .unwrap()
            .starts_with("Spoken call-outs: off."));
        h.clear();
        h.press(Key::Return);
        assert_eq!(h.spoken(), ["Spoken call-outs: on. Each cue is named."]);
        assert!(Records::load::<bool>(&mut h.app.ctx, KEY));
        h.clear();
        h.press(Key::Return);
        assert!(
            h.spoken()[0].starts_with("Spoken call-outs: off."),
            "{:?}",
            h.spoken()
        );
        h.clear();
        h.press(Key::Right);
        assert!(
            h.spoken()[0].starts_with("Spoken call-outs: on."),
            "{:?}",
            h.spoken()
        );
        h.press(Key::F1);
        assert_eq!(h.spoken().last().unwrap(), "Speak each cue.");
    }
}
