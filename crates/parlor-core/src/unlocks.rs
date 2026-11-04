//! Unlockables: things a player earns by playing, saved per game and read
//! back on a shared Unlocks menu.
//!
//! A game lists every unlockable it has as [`Unlock`]s, each with a stable
//! id, a group ("Diary pages", "Curses", "Music"), a name, the rule that
//! earns it in words, and what it is. Locked ones are listed too, with
//! their rule, so a player always knows what is left and how to get it.
//!
//! The game decides when a rule is met and calls [`Unlocks::earn`], which
//! saves at once and says whether it was new, so the game can announce it
//! with [`earned_line`]. The title's Unlocks row ([`title_row`]) opens
//! [`UnlocksMenu`]: one row per group with its count, then each group's
//! entries. Enter on an earned entry runs the game's
//! [`UnlockGame::use_unlock`]: read a diary page, play a tune, switch a
//! curse on or off.

use std::collections::BTreeSet;
use std::marker::PhantomData;

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::Ctx;
use serde::{Deserialize, Serialize};

use crate::menus::{labels, Title, TitleGame, TitleRow};
use crate::{Parlor, Records};

/// One thing a player can earn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unlock {
    /// Stable save id, unique within the game: "diary-3". Never change one
    /// that has shipped.
    pub id: &'static str,
    /// The group it is listed under: "Diary pages".
    pub group: &'static str,
    /// Its name, two to five words: "Diary page 3".
    pub name: String,
    /// How to earn it, as a sentence: "Clear floor 15."
    pub rule: String,
    /// What it is or gives, as a sentence: "A page of the housekeeper's diary."
    pub about: String,
}

impl Unlock {
    pub fn new(
        id: &'static str,
        group: &'static str,
        name: impl Into<String>,
        rule: impl Into<String>,
        about: impl Into<String>,
    ) -> Unlock {
        Unlock {
            id,
            group,
            name: name.into(),
            rule: rule.into(),
            about: about.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct Saved {
    #[serde(default)]
    earned: BTreeSet<String>,
}

/// The saved unlocks of every game.
pub struct Unlocks;

impl Unlocks {
    /// The records key a game's unlocks are saved under.
    pub fn key(game: &str) -> String {
        format!("{game}/unlocks")
    }

    /// The ids a game's player has earned.
    pub fn earned(ctx: &mut Ctx<Parlor>, game: &str) -> BTreeSet<String> {
        let saved: Saved = Records::load(ctx, &Self::key(game));
        saved.earned
    }

    pub fn has(ctx: &mut Ctx<Parlor>, game: &str, id: &str) -> bool {
        Self::earned(ctx, game).contains(id)
    }

    /// Earns an unlock and saves at once. True when it is new.
    pub fn earn(ctx: &mut Ctx<Parlor>, game: &str, id: &str) -> bool {
        let mut saved: Saved = Records::load(ctx, &Self::key(game));
        let new = saved.earned.insert(id.to_string());
        if new {
            Records::store(ctx, &Self::key(game), &saved);
        }
        new
    }

    /// Earns every id in `ids` that `list` holds, and returns the new ones'
    /// names, in the list's order.
    pub fn earn_all(
        ctx: &mut Ctx<Parlor>,
        game: &str,
        list: &[Unlock],
        ids: &[&str],
    ) -> Vec<String> {
        let mut saved: Saved = Records::load(ctx, &Self::key(game));
        let mut names = Vec::new();
        for u in list {
            if ids.contains(&u.id) && saved.earned.insert(u.id.to_string()) {
                names.push(u.name.clone());
            }
        }
        if !names.is_empty() {
            Records::store(ctx, &Self::key(game), &saved);
        }
        names
    }

    /// How many of `list` are earned.
    pub fn count(ctx: &mut Ctx<Parlor>, game: &str, list: &[Unlock]) -> usize {
        let earned = Self::earned(ctx, game);
        list.iter().filter(|u| earned.contains(u.id)).count()
    }
}

/// The line that announces new unlocks, or `None` for none:
/// "Unlocked: Diary page 3." or "Unlocked: Nightmare and Diary page 5."
pub fn earned_line(names: &[String]) -> Option<String> {
    match names {
        [] => None,
        [one] => Some(format!("Unlocked: {one}.")),
        [rest @ .., last] => Some(format!("Unlocked: {} and {last}.", rest.join(", "))),
    }
}

/// What a game gives the shared Unlocks menu.
pub trait UnlockGame: 'static {
    /// The game's id, as its unlocks are saved under.
    const ID: &'static str;
    /// The game's name, for the Back row's help.
    const NAME: &'static str;
    /// The menu's `State::name`: the title's name plus `_unlocks`.
    const MENU_NAME: &'static str;

    /// Every unlockable, earned or not, grouped and in the order to list.
    fn unlocks(ctx: &mut Ctx<Parlor>) -> Vec<Unlock>;

    /// Enter on an earned entry. By default, says what it is.
    fn use_unlock(ctx: &mut Ctx<Parlor>, unlock: &Unlock) {
        ctx.say(unlock.about.clone());
    }

    /// A value an earned entry holds, read after its name: "on" or "off"
    /// for a curse. `None` for most.
    fn state(_ctx: &mut Ctx<Parlor>, _unlock: &Unlock) -> Option<String> {
        None
    }

    /// Starts the menu's music, on entering and on every return.
    fn music(_ctx: &mut Ctx<Parlor>) {}
}

/// "Unlocks, 3 of 40": the title's row.
pub fn row_label<G: UnlockGame>(ctx: &mut Ctx<Parlor>) -> String {
    let list = G::unlocks(ctx);
    let earned = Unlocks::count(ctx, G::ID, &list);
    labels::unlocks(earned, list.len())
}

/// The title's Unlocks row, for `TitleGame::extras`.
pub fn title_row<T: TitleGame, G: UnlockGame>(ctx: &mut Ctx<Parlor>) -> TitleRow<T> {
    MenuItem::new(
        row_label::<G>(ctx),
        |_s: &mut Title<T>, ctx: &mut Ctx<Parlor>| ctx.push(UnlocksMenu::<G>::new()),
    )
    .help("Everything you can earn, what you have, and how to earn the rest.")
}

/// The groups in the order the list first names them.
fn groups(list: &[Unlock]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for u in list {
        if !out.contains(&u.group) {
            out.push(u.group);
        }
    }
    out
}

/// The Unlocks menu: one row per group with its count, then Back.
pub struct UnlocksMenu<G: UnlockGame> {
    menu: MenuCore<Self, Parlor>,
    _game: PhantomData<G>,
}

impl<G: UnlockGame> UnlocksMenu<G> {
    pub fn new() -> Self {
        UnlocksMenu {
            menu: MenuCore::new(labels::UNLOCKS).with_intro_help(
                "Up and Down choose a group, Enter lists what is in it, Escape goes back.",
            ),
            _game: PhantomData,
        }
    }
}

impl<G: UnlockGame> Default for UnlocksMenu<G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<G: UnlockGame> Menu for UnlocksMenu<G> {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        G::MENU_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        G::music(ctx);
        hearken::menu::menu_enter(self, ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        G::music(ctx);
        self.refresh(ctx, true);
        self.announce(ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let list = G::unlocks(ctx);
        let earned = Unlocks::earned(ctx, G::ID);
        let mut items: Vec<MenuItem<Self, Parlor>> = groups(&list)
            .into_iter()
            .map(|group| {
                let of = list.iter().filter(|u| u.group == group).count();
                let have = list
                    .iter()
                    .filter(|u| u.group == group && earned.contains(u.id))
                    .count();
                let next = list
                    .iter()
                    .find(|u| u.group == group && !earned.contains(u.id));
                let help = match next {
                    Some(u) => format!("Next: {}, {}", u.name, lower_first(&u.rule)),
                    None => "Every one earned.".to_string(),
                };
                MenuItem::new(
                    format!("{group}, {have} of {of}"),
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.push(UnlockGroup::<G>::new(group))
                    },
                )
                .help(help)
            })
            .collect();
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help(format!("Return to the {} title.", G::NAME)),
        );
        items
    }
}

/// "clear floor 15." from "Clear floor 15.", to follow a comma.
fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// The `State::name` of a group's list.
pub const GROUP_NAME: &str = "unlock_group";

/// One group's entries: earned ones by name, the rest locked with their
/// rule on F1 and Enter.
pub struct UnlockGroup<G: UnlockGame> {
    group: &'static str,
    menu: MenuCore<Self, Parlor>,
    _game: PhantomData<G>,
}

impl<G: UnlockGame> UnlockGroup<G> {
    pub fn new(group: &'static str) -> Self {
        UnlockGroup {
            group,
            menu: MenuCore::new(group).with_intro_help(
                "Up and Down move through the list, Enter uses what you have earned or says how to earn it, F1 says the same, Escape goes back.",
            ),
            _game: PhantomData,
        }
    }
}

impl<G: UnlockGame> Menu for UnlockGroup<G> {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        GROUP_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let list: Vec<Unlock> = G::unlocks(ctx)
            .into_iter()
            .filter(|u| u.group == self.group)
            .collect();
        let earned = Unlocks::earned(ctx, G::ID);
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for u in list {
            if earned.contains(u.id) {
                let label = match G::state(ctx, &u) {
                    Some(state) => format!("{}: {state}", u.name),
                    None => u.name.clone(),
                };
                let help = u.about.clone();
                items.push(
                    MenuItem::new(label, move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        G::use_unlock(ctx, &u);
                        if G::state(ctx, &u).is_some() {
                            s.refresh(ctx, true);
                            s.speak_current(ctx);
                        }
                    })
                    .select_sound(None)
                    .help(help),
                );
            } else {
                let rule = u.rule.clone();
                items.push(
                    MenuItem::new(
                        labels::locked(&u.name),
                        move |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.say(rule.clone()),
                    )
                    .select_sound(None)
                    .help(u.rule.clone()),
                );
            }
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the list of groups."),
        );
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::testing::{rows, Base};
    use hearken::{Headless, Key};

    struct Dummy;

    impl UnlockGame for Dummy {
        const ID: &'static str = "dummy";
        const NAME: &'static str = "Dummy";
        const MENU_NAME: &'static str = "dummy_unlocks";

        fn unlocks(_ctx: &mut Ctx<Parlor>) -> Vec<Unlock> {
            vec![
                Unlock::new(
                    "page-1",
                    "Diary pages",
                    "Diary page 1",
                    "Clear floor 5.",
                    "The first page.",
                ),
                Unlock::new(
                    "page-2",
                    "Diary pages",
                    "Diary page 2",
                    "Clear floor 10.",
                    "The second page.",
                ),
                Unlock::new(
                    "curse",
                    "Curses",
                    "Lights out",
                    "Clear a floor with the lights out.",
                    "Things call out more softly.",
                ),
            ]
        }

        fn use_unlock(ctx: &mut Ctx<Parlor>, unlock: &Unlock) {
            if unlock.id == "curse" {
                let on: bool = Records::load(ctx, "dummy/curse");
                Records::store(ctx, "dummy/curse", &!on);
            } else {
                ctx.say(unlock.about.clone());
            }
        }

        fn state(ctx: &mut Ctx<Parlor>, unlock: &Unlock) -> Option<String> {
            (unlock.id == "curse").then(|| {
                let on: bool = Records::load(ctx, "dummy/curse");
                if on { "on" } else { "off" }.to_string()
            })
        }
    }

    fn open() -> Headless<Parlor> {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(UnlocksMenu::<Dummy>::new());
        h.frame(0.0);
        h
    }

    #[test]
    fn earning_saves_once_and_counts() {
        let mut h = open();
        let ctx = &mut h.app.ctx;
        assert!(!Unlocks::has(ctx, "dummy", "page-1"));
        assert!(Unlocks::earn(ctx, "dummy", "page-1"));
        assert!(!Unlocks::earn(ctx, "dummy", "page-1"));
        assert!(Unlocks::has(ctx, "dummy", "page-1"));
        let list = Dummy::unlocks(ctx);
        assert_eq!(Unlocks::count(ctx, "dummy", &list), 1);
        let names = Unlocks::earn_all(ctx, "dummy", &list, &["page-1", "page-2", "curse"]);
        assert_eq!(names, ["Diary page 2", "Lights out"]);
        assert_eq!(Unlocks::count(ctx, "dummy", &list), 3);
    }

    #[test]
    fn earned_lines_name_what_is_new() {
        assert_eq!(earned_line(&[]), None);
        assert_eq!(
            earned_line(&["Nightmare".to_string()]).unwrap(),
            "Unlocked: Nightmare."
        );
        assert_eq!(
            earned_line(&["A".to_string(), "B".to_string(), "C".to_string()]).unwrap(),
            "Unlocked: A, B and C."
        );
    }

    #[test]
    fn groups_count_and_locked_entries_say_their_rule() {
        let mut h = open();
        assert_eq!(rows(&h), ["Diary pages, 0 of 2", "Curses, 0 of 1", "Back"]);
        h.clear();
        h.press(Key::F1);
        assert_eq!(h.spoken(), ["Next: Diary page 1, clear floor 5."]);
        h.press(Key::Return);
        assert_eq!(
            rows(&h),
            ["Diary page 1, locked", "Diary page 2, locked", "Back"]
        );
        h.clear();
        h.press(Key::Return);
        assert_eq!(h.spoken(), ["Clear floor 5."]);
    }

    #[test]
    fn earned_entries_are_used_and_states_toggle() {
        let mut h = open();
        Unlocks::earn(&mut h.app.ctx, "dummy", "page-1");
        Unlocks::earn(&mut h.app.ctx, "dummy", "curse");
        h.press(Key::Escape);
        h.app.ctx.push(UnlocksMenu::<Dummy>::new());
        h.frame(0.0);
        assert_eq!(rows(&h), ["Diary pages, 1 of 2", "Curses, 1 of 1", "Back"]);
        h.press(Key::Return);
        h.clear();
        h.press(Key::Return);
        assert_eq!(h.spoken(), ["The first page."]);
        h.press(Key::Escape);
        h.press(Key::Down);
        h.press(Key::Return);
        assert_eq!(rows(&h), ["Lights out: off", "Back"]);
        h.press(Key::Return);
        assert_eq!(rows(&h), ["Lights out: on", "Back"]);
        h.press(Key::Escape);
        h.press(Key::Escape);
        assert_eq!(h.app.top_name(), "base");
    }
}
