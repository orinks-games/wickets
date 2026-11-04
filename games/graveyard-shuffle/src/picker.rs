//! The night select and the practice room's song list.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Ctx, UiSound};
use parlor_core::menus::labels;
use parlor_core::{challenges, Date, Parlor, Records};

use crate::play::{Floor, Mode};
use crate::progress::{self, dated, Progress};
use crate::rules::stars_words;
use crate::songs::{night_of, NIGHTS, SONGS};
use crate::sound;

/// The set select's `State::name`.
pub const SETS_NAME: &str = "graveyard_shuffle_sets";
/// The practice room's tune list.
pub const PRACTICE_NAME: &str = "graveyard_shuffle_practice";

/// "Night 4, Doctor Volt's laboratory, opens only around Halloween, ..."
pub fn waits_words(number: u32) -> String {
    let set = &NIGHTS[(number as usize - 1).min(NIGHTS.len() - 1)];
    format!(
        "{} opens only around Halloween, from the 17th of October to the 1st of November.",
        set.title()
    )
}

/// "Night 2, the crypt, 4 of 9 stars".
pub fn set_label(p: &Progress, number: u32, today: Date) -> String {
    let set = &NIGHTS[(number as usize - 1).min(NIGHTS.len() - 1)];
    if !p.night_open(number, today) {
        return labels::locked(&set.title());
    }
    let stars: u32 = set.songs().iter().map(|t| p.stars_for(t.id)).sum();
    format!("{}, {stars} of 9 stars", set.title())
}

/// F1 on a set's row.
pub fn set_help(ctx: &mut Ctx<Parlor>, p: &Progress, number: u32) -> String {
    let set = &NIGHTS[(number as usize - 1).min(NIGHTS.len() - 1)];
    if !p.set_open(number) {
        return format!(
            "Earn a star on every song of night {} to open {}.",
            number - 1,
            set.name
        );
    }
    if !dated(number, challenges::today(ctx)) {
        return waits_words(number);
    }
    let names: Vec<&str> = set.songs().iter().map(|t| t.name).collect();
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
            menu: MenuCore::new("Night select")
                .with_intro_help("Up and Down choose a night, Enter dances it, Escape goes back."),
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
        let today = challenges::today(ctx);
        let mut items: Vec<MenuItem<Self, Parlor>> = NIGHTS
            .iter()
            .map(|set| {
                let n = set.number;
                let open = p.night_open(n, today);
                let help = set_help(ctx, &p, n);
                MenuItem::new(
                    set_label(&p, n, today),
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
            .help("Return to the Graveyard Shuffle title."),
        );
        items
    }
}

/// "Boneyard Boogie, 2 stars", or locked.
pub fn tune_label(p: &Progress, index: usize, today: Date) -> String {
    let t = &SONGS[index];
    if !p.night_open(night_of(index).number, today) {
        return labels::locked(t.name);
    }
    format!("{}, {}", t.name, stars_words(p.stars_for(t.id)))
}

pub struct TuneSelect {
    menu: MenuCore<Self, Parlor>,
}

impl TuneSelect {
    pub fn new() -> TuneSelect {
        TuneSelect {
            menu: MenuCore::new("Practice room").with_intro_help(
                "Up and Down choose a song, Enter practises it, Escape goes back. The song waits for each monster until you dance its move.",
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
        let today = challenges::today(ctx);
        let mut items: Vec<MenuItem<Self, Parlor>> = (0..SONGS.len())
            .map(|i| {
                let set = night_of(i);
                let open = p.night_open(set.number, today);
                let t = &SONGS[i];
                let help = if open {
                    format!(
                        "From {}, {} beats a minute.",
                        set.title_in_sentence(),
                        t.bpm
                    )
                } else if p.set_open(set.number) {
                    waits_words(set.number)
                } else {
                    format!(
                        "Earn a star on every song of night {} to practise the songs of {}.",
                        set.number - 1,
                        set.name
                    )
                };
                let locked_line = help.clone();
                MenuItem::new(
                    tune_label(&p, i, today),
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
            .help("Return to the Graveyard Shuffle title."),
        );
        items
    }
}
