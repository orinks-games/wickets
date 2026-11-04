//! The set select and the practice room's tune list.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Ctx, UiSound};
use parlor_core::menus::labels;
use parlor_core::{Parlor, Records};

use crate::play::{Floor, Mode};
use crate::progress::{self, Progress};
use crate::rules::stars_words;
use crate::sound;
use crate::tunes::{set_of, SETS, TUNES};

/// The set select's `State::name`.
pub const SETS_NAME: &str = "clover_jig_sets";
/// The practice room's tune list.
pub const PRACTICE_NAME: &str = "clover_jig_practice";

/// "Set 2, the crossroads, 4 of 9 stars".
pub fn set_label(p: &Progress, number: u32) -> String {
    let set = &SETS[(number as usize - 1).min(SETS.len() - 1)];
    if !p.set_open(number) {
        return labels::locked(&set.title());
    }
    let stars: u32 = set.tunes().iter().map(|t| p.stars_for(t.id)).sum();
    format!("{}, {stars} of 9 stars", set.title())
}

/// F1 on a set's row.
pub fn set_help(ctx: &mut Ctx<Parlor>, p: &Progress, number: u32) -> String {
    let set = &SETS[(number as usize - 1).min(SETS.len() - 1)];
    if !p.set_open(number) {
        return format!(
            "Earn a star on every tune of set {} to open {}.",
            number - 1,
            set.name
        );
    }
    let names: Vec<&str> = set.tunes().iter().map(|t| t.name).collect();
    let best = match Records::best(ctx, crate::ID, &progress::set_board(number)) {
        Some(b) => format!(" Best {b}."),
        None => String::new(),
    };
    format!("{} {}.{best}", set.scene, names.join(", "))
}

pub struct SetSelect {
    menu: MenuCore<Self, Parlor>,
}

impl SetSelect {
    pub fn new() -> SetSelect {
        SetSelect {
            menu: MenuCore::new("Set select")
                .with_intro_help("Up and Down choose a set, Enter dances it, Escape goes back."),
        }
    }
}

impl Default for SetSelect {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for SetSelect {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        SETS_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
        self.refresh(ctx, true);
        self.announce(ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let p = Progress::load(ctx);
        let mut items: Vec<MenuItem<Self, Parlor>> = SETS
            .iter()
            .map(|set| {
                let n = set.number;
                let open = p.set_open(n);
                let help = set_help(ctx, &p, n);
                MenuItem::new(
                    set_label(&p, n),
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        if open {
                            let f = Floor::open(ctx, Mode::Set(n));
                            ctx.push(f);
                        } else {
                            ctx.play_ui(UiSound::Error);
                            let p = Progress::load(ctx);
                            let line = set_help(ctx, &p, n);
                            ctx.say(line);
                        }
                    },
                )
                .help(help)
            })
            .collect();
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(UiSound::Back);
                ctx.pop()
            })
            .help("Return to the Clover Jig title."),
        );
        items
    }
}

/// "The Turf Fire, a reel, 2 stars" or "..., locked".
pub fn tune_label(p: &Progress, index: usize) -> String {
    let t = &TUNES[index];
    if !p.set_open(set_of(index).number) {
        return labels::locked(&t.title());
    }
    format!("{}, {}", t.title(), stars_words(p.stars_for(t.id)))
}

pub struct TuneSelect {
    menu: MenuCore<Self, Parlor>,
}

impl TuneSelect {
    pub fn new() -> TuneSelect {
        TuneSelect {
            menu: MenuCore::new("Practice room").with_intro_help(
                "Up and Down choose a tune, Enter practises it, Escape goes back. The tune waits at each step until you dance it.",
            ),
        }
    }
}

impl Default for TuneSelect {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for TuneSelect {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        PRACTICE_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
        self.refresh(ctx, true);
        self.announce(ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let p = Progress::load(ctx);
        let mut items: Vec<MenuItem<Self, Parlor>> = (0..TUNES.len())
            .map(|i| {
                let set = set_of(i);
                let open = p.set_open(set.number);
                let t = &TUNES[i];
                let help = if open {
                    format!(
                        "From {}, {} beats a minute.",
                        set.title().to_lowercase(),
                        t.bpm
                    )
                } else {
                    format!(
                        "Earn a star on every tune of set {} to practise the tunes of {}.",
                        set.number - 1,
                        set.name
                    )
                };
                let locked_line = help.clone();
                MenuItem::new(
                    tune_label(&p, i),
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        if open {
                            let f = Floor::open(ctx, Mode::Practice(i));
                            ctx.push(f);
                        } else {
                            ctx.play_ui(UiSound::Error);
                            ctx.say(locked_line.clone());
                        }
                    },
                )
                .help(help)
            })
            .collect();
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(UiSound::Back);
                ctx.pop()
            })
            .help("Return to the Clover Jig title."),
        );
        items
    }
}
