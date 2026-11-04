//! Story and event games: the categories and single games the player wants
//! story mode and the events to offer. Anything left out stays in Play
//! and plays as before; story sittings, the lamps, Happening today and the
//! lobby's event line just stop naming it.
//!
//! Saved in records (`parlor/preferences`) as what is left out, so a new
//! category or game starts on, and everything is on by default. At least
//! one game always stays on.
//!
//! The launcher lists its games with [`register`], since this crate does
//! not know them.

use std::sync::OnceLock;

use hearken::menu::{Label, Menu, MenuCore, MenuItem};
use hearken::screens::{SettingsRow, SettingsScreen};
use hearken::{Ctx, UiSound};
use serde::{Deserialize, Serialize};

use crate::menus::labels;
use crate::{Category, GameInfo, Parlor, Records};

/// The records key the preferences are saved under.
pub const KEY: &str = "parlor/preferences";
/// The Settings row's id.
pub const ROW_ID: &str = "preferences";
/// The screen's name, as the Settings row and help say it.
pub const LABEL: &str = "Story and event games";

const LAST_ONE: &str = "No change. Keep at least one game on for Story and events.";

/// What is saved: the categories and games left out, by id.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    /// Categories left out.
    #[serde(default)]
    pub left_out: Vec<String>,
    /// Single games left out, in a category that is on.
    #[serde(default)]
    pub games_left_out: Vec<String>,
}

impl Saved {
    fn has_category(&self, category: Category) -> bool {
        !self.left_out.iter().any(|id| id == category.id())
    }

    /// Whether story and events offer `g`. A game is judged by the
    /// category the preferences list it in; one they do not list, such as
    /// a holiday special, by the category it is listed in today.
    pub fn shows(&self, g: &GameInfo) -> bool {
        let category = games()
            .iter()
            .find(|x| x.id == g.id)
            .map_or(g.category, |x| x.category);
        self.has_category(category) && !self.games_left_out.iter().any(|id| id == g.id)
    }
}

static GAMES: OnceLock<fn() -> Vec<GameInfo>> = OnceLock::new();

/// Lists the games the preferences offer, by category. The launcher calls
/// it once; later calls are ignored.
pub fn register(games: fn() -> Vec<GameInfo>) {
    let _ = GAMES.set(games);
}

fn games() -> Vec<GameInfo> {
    GAMES.get().map(|f| f()).unwrap_or_default()
}

fn games_in(category: Category) -> Vec<GameInfo> {
    games()
        .into_iter()
        .filter(|g| g.category == category)
        .collect()
}

fn saved(ctx: &Ctx<Parlor>) -> Saved {
    Records::peek(ctx, KEY)
}

/// The preferences, read from records (loading them first if need be).
pub fn load(ctx: &mut Ctx<Parlor>) -> Saved {
    Records::load(ctx, KEY)
}

/// Whether story and events offer `g`, from records already read.
pub fn shows(ctx: &Ctx<Parlor>, g: &GameInfo) -> bool {
    saved(ctx).shows(g)
}

/// Whether the category is on, from records already read.
pub fn likes(ctx: &Ctx<Parlor>, category: Category) -> bool {
    saved(ctx).has_category(category)
}

/// Saves `s` unless it leaves no game on. Returns whether it saved.
fn store(ctx: &mut Ctx<Parlor>, s: &Saved) -> bool {
    let all = games();
    if !all.is_empty() && !all.iter().any(|g| s.shows(g)) {
        return false;
    }
    Records::store(ctx, KEY, s);
    true
}

/// Turns a whole category on or off. Turning it on brings back every game
/// in it. Returns false, changing nothing, when that would leave no game.
pub fn set(ctx: &mut Ctx<Parlor>, category: Category, on: bool) -> bool {
    let mut s = load(ctx);
    let id = category.id().to_string();
    s.left_out.retain(|c| *c != id);
    let ids: Vec<&str> = games_in(category).iter().map(|g| g.id).collect();
    s.games_left_out.retain(|g| !ids.contains(&g.as_str()));
    if !on {
        s.left_out.push(id);
    }
    store(ctx, &s)
}

/// Turns one game on or off. Turning on a game in a category that is off
/// turns the category on with only that game. Returns false, changing
/// nothing, when that would leave no game.
pub fn set_game(ctx: &mut Ctx<Parlor>, g: &GameInfo, on: bool) -> bool {
    let mut s = load(ctx);
    if on && !s.has_category(g.category) {
        s.left_out.retain(|c| c != g.category.id());
        for other in games_in(g.category) {
            if !s.games_left_out.iter().any(|x| x == other.id) {
                s.games_left_out.push(other.id.to_string());
            }
        }
    }
    s.games_left_out.retain(|x| x != g.id);
    if !on {
        s.games_left_out.push(g.id.to_string());
    }
    // A category with every game off is the category off.
    let category = games_in(g.category);
    if !category.is_empty() && category.iter().all(|x| !s.shows(x)) {
        s.left_out.push(g.category.id().to_string());
        s.games_left_out
            .retain(|x| !category.iter().any(|c| c.id == x.as_str()));
    }
    store(ctx, &s)
}

/// "all on", "1 game off" or "5 games off".
fn summary(ctx: &Ctx<Parlor>) -> String {
    let s = saved(ctx);
    let off = games().iter().filter(|g| !s.shows(g)).count();
    match off {
        0 => "all on".to_string(),
        1 => "1 game off".to_string(),
        n => format!("{n} games off"),
    }
}

/// The shared Settings row, which opens the preferences.
pub fn row() -> SettingsRow<Parlor> {
    SettingsRow::new(
        ROW_ID,
        MenuItem::new(
            Label::dynamic(|_s: &SettingsScreen<Parlor>, ctx: &Ctx<Parlor>| {
                format!("{LABEL}: {}", summary(ctx))
            }),
            |_s: &mut SettingsScreen<Parlor>, ctx: &mut Ctx<Parlor>| {
                ctx.push(PreferencesMenu::new())
            },
        )
        .help(
            "Choose which games Story and events may offer. Every game remains available in Play.",
        ),
    )
}

/// "on", "off" or "1 of 2 on".
fn category_state(ctx: &Ctx<Parlor>, category: Category) -> String {
    let s = saved(ctx);
    let all = games_in(category);
    let on = all.iter().filter(|g| s.shows(g)).count();
    if !s.has_category(category) || on == 0 {
        "off".to_string()
    } else if on == all.len() {
        "on".to_string()
    } else {
        format!("{on} of {} on", all.len())
    }
}

/// One row a category: Enter lists its games, Left and Right turn the
/// whole category on or off.
pub struct PreferencesMenu {
    menu: MenuCore<Self, Parlor>,
}

impl PreferencesMenu {
    pub fn new() -> Self {
        PreferencesMenu {
            menu: MenuCore::new(LABEL).with_intro_help(
                "Up and Down choose a category, Enter lists its games, Left and Right turn it on or off.",
            ),
        }
    }
}

impl Default for PreferencesMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for PreferencesMenu {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "preferences"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = Category::ALL
            .into_iter()
            .filter(|&c| !games_in(c).is_empty())
            .map(|category| {
                MenuItem::new(
                    format!("{}: {}", category.label(), category_state(ctx, category)),
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.push(CategoryMenu::new(category))
                    },
                )
                .help(format!(
                    "Enter lists the {}. Left and Right turn them all on or off.",
                    category.label().to_lowercase()
                ))
                // The menu reads the row back after Left and Right.
                .adjustable(move |s: &mut Self, ctx: &mut Ctx<Parlor>, _delta| {
                    let on = category_state(ctx, category) == "off";
                    if set(ctx, category, on) {
                        s.refresh(ctx, true);
                    }
                })
            })
            .collect();
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(UiSound::Back);
                ctx.pop()
            })
            .help("Return to Settings."),
        );
        items
    }
}

/// One category's games: a row for the whole category, then one a game.
/// Enter, Left and Right turn a row on or off.
pub struct CategoryMenu {
    category: Category,
    menu: MenuCore<Self, Parlor>,
}

impl CategoryMenu {
    pub fn new(category: Category) -> Self {
        CategoryMenu {
            category,
            menu: MenuCore::new(category.label()).with_intro_help(
                "Up and Down choose a game, Enter turns it on or off, Escape goes back.",
            ),
        }
    }

    fn flip_all(&mut self, ctx: &mut Ctx<Parlor>, speak: bool) {
        let category = self.category;
        let on = category_state(ctx, category) != "on";
        if !set(ctx, category, on) {
            if speak {
                ctx.say(LAST_ONE);
            }
            return;
        }
        self.refresh(ctx, true);
        if speak {
            let line = if on {
                format!("All {}: on.", category.label().to_lowercase())
            } else {
                format!(
                    "All {}: off. Still in Play.",
                    category.label().to_lowercase()
                )
            };
            ctx.say(line);
        }
    }

    fn flip(&mut self, ctx: &mut Ctx<Parlor>, g: GameInfo, speak: bool) {
        let on = !shows(ctx, &g);
        if !set_game(ctx, &g, on) {
            if speak {
                ctx.say(LAST_ONE);
            }
            return;
        }
        self.refresh(ctx, true);
        if speak {
            let line = if on {
                format!("{}: on.", g.name)
            } else {
                format!("{}: off. Still in Play.", g.name)
            };
            ctx.say(line);
        }
    }
}

impl Menu for CategoryMenu {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "preferences_category"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let category = self.category;
        let lower = category.label().to_lowercase();
        let mut items = vec![MenuItem::new(
            format!("All {lower}: {}", category_state(ctx, category)),
            |s: &mut Self, ctx: &mut Ctx<Parlor>| s.flip_all(ctx, true),
        )
        .help(format!(
            "Include or exclude all {lower} from Story and events. They stay in Play."
        ))
        .adjustable(|s: &mut Self, ctx: &mut Ctx<Parlor>, _delta| s.flip_all(ctx, false))];
        for g in games_in(category) {
            let state = if shows(ctx, &g) { "on" } else { "off" };
            items.push(
                MenuItem::new(
                    format!("{}: {state}", g.name),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| s.flip(ctx, g, true),
                )
                .help(format!(
                    "When on, Story and events may offer {}. It always stays in Play.",
                    g.name
                ))
                .adjustable(move |s: &mut Self, ctx: &mut Ctx<Parlor>, _delta| {
                    s.flip(ctx, g, false)
                }),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(UiSound::Back);
                ctx.pop()
            })
            .help("Return to the categories."),
        );
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::testing::Base;
    use hearken::{Headless, Key};

    fn game(id: &'static str, name: &'static str, category: Category) -> GameInfo {
        GameInfo {
            id,
            name,
            category,
            blurb: "",
            open: |_| {},
            challenge: None,
            event: None,
            event_done: None,
            tutorial: |_| crate::Lesson::new("dummy", "Dummy", "Hi"),
            settings: |_| Vec::new(),
            story: crate::story::StoryTags {
                times: &[],
                seasons: &[],
            },
        }
    }

    fn roster() -> Vec<GameInfo> {
        vec![
            game("a1", "Alpha", Category::Action),
            game("t1", "Dice", Category::Table),
            game("t2", "Patience", Category::Table),
        ]
    }

    fn open() -> Headless<Parlor> {
        register(roster);
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(PreferencesMenu::new());
        h.frame(0.0);
        h
    }

    fn on(h: &Headless<Parlor>) -> Vec<&'static str> {
        roster()
            .into_iter()
            .filter(|g| shows(&h.app.ctx, g))
            .map(|g| g.id)
            .collect()
    }

    #[test]
    fn everything_starts_on_and_lists_categories_with_games() {
        let h = open();
        assert_eq!(on(&h), ["a1", "t1", "t2"]);
        let rows = crate::menus::testing::rows(&h);
        assert_eq!(rows, ["Action games: on", "Table games: on", "Back"]);
    }

    #[test]
    fn one_game_can_go_and_its_category_stays() {
        let mut h = open();
        h.press(Key::Down);
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "preferences_category");
        h.press(Key::End);
        h.press(Key::Up);
        h.clear();
        h.press(Key::Return);
        assert_eq!(h.spoken(), ["Patience: off. Still in Play."]);
        assert_eq!(on(&h), ["a1", "t1"]);
        h.press(Key::Escape);
        let rows = crate::menus::testing::rows(&h);
        assert_eq!(rows[1], "Table games: 1 of 2 on");
        let s: Saved = Records::load(&mut h.app.ctx, KEY);
        assert_eq!(s.games_left_out, ["t2"]);
    }

    #[test]
    fn left_and_right_turn_a_whole_category() {
        let mut h = open();
        h.press(Key::Down);
        h.clear();
        h.press(Key::Right);
        assert!(
            h.spoken()[0].starts_with("Table games: off"),
            "{:?}",
            h.spoken()
        );
        assert_eq!(on(&h), ["a1"]);
        h.press(Key::Left);
        assert_eq!(on(&h), ["a1", "t1", "t2"]);
    }

    #[test]
    fn a_game_turned_on_in_an_off_category_comes_back_alone() {
        let mut h = open();
        assert!(set(&mut h.app.ctx, Category::Table, false));
        let dice = roster()[1];
        assert!(set_game(&mut h.app.ctx, &dice, true));
        assert_eq!(on(&h), ["a1", "t1"]);
        // Its last game off turns the category off.
        assert!(set_game(&mut h.app.ctx, &dice, false));
        assert!(!likes(&h.app.ctx, Category::Table));
    }

    #[test]
    fn the_last_game_stays_on() {
        let mut h = open();
        assert!(set(&mut h.app.ctx, Category::Table, false));
        let alpha = roster()[0];
        assert!(!set_game(&mut h.app.ctx, &alpha, false));
        assert!(!set(&mut h.app.ctx, Category::Action, false));
        assert_eq!(on(&h), ["a1"]);
        h.press(Key::Return);
        h.clear();
        h.press(Key::Down);
        h.press(Key::Return);
        assert_eq!(h.spoken().last().unwrap(), LAST_ONE);
    }
}
