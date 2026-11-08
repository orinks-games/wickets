//! The pond select: every pond with its stars and best, or what opens it.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::Ctx;
use parlor_core::menus::labels;
use parlor_core::{Parlor, Records};

use crate::play::{Mode, SkateScreen};
use crate::ponds::PONDS;
use crate::progress::{pond_board, Progress};
use crate::rules::{points_words, stars_words};
use crate::ID;

pub struct Ponds {
    menu: MenuCore<Self, Parlor>,
    seed: Option<u64>,
}

impl Ponds {
    pub fn new(seed: Option<u64>) -> Ponds {
        Ponds {
            menu: MenuCore::new("Ponds")
                .with_intro_help("Up and Down choose a pond, Enter skates it, Escape goes back."),
            seed,
        }
    }
}

/// The best on pond `n`, in words.
pub fn best_words(ctx: &mut Ctx<Parlor>, n: u32) -> String {
    match Records::best(ctx, ID, &pond_board(n)) {
        Some(b) => format!("best {}", points_words(b as u32)),
        None => "not skated yet".to_string(),
    }
}

/// Starts a skate in `mode`, seeded for tests when `seed` is set.
pub fn skate(ctx: &mut Ctx<Parlor>, mode: Mode, seed: Option<u64>) {
    let s = match seed {
        Some(seed) => SkateScreen::with_seed(ctx, mode, seed),
        None => SkateScreen::open(ctx, mode),
    };
    ctx.push(s);
}

impl Menu for Ponds {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "snowfall_skate_ponds"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let p = Progress::load(ctx);
        let mut items = Vec::new();
        for pond in &PONDS {
            let n = pond.number;
            if p.pond_open(n) {
                let best = best_words(ctx, n);
                items.push(
                    MenuItem::new(
                        format!("{}, {}, {best}", pond.title(), stars_words(p.stars_for(n))),
                        move |s: &mut Ponds, ctx: &mut Ctx<Parlor>| {
                            skate(ctx, Mode::Pond(n), s.seed)
                        },
                    )
                    .help(format!(
                        "{} Stars at {}, {} and {}.",
                        pond.scene,
                        points_words(pond.marks[0]),
                        points_words(pond.marks[1]),
                        points_words(pond.marks[2])
                    )),
                );
            } else {
                let text = format!(
                    "{} opens with a star on {}.",
                    pond.subject(),
                    crate::ponds::pond(n - 1).name
                );
                items.push(
                    MenuItem::new(
                        labels::locked(&pond.title()),
                        move |_s: &mut Ponds, ctx: &mut Ctx<Parlor>| ctx.say(text.clone()),
                    )
                    .select_sound(None)
                    .help(format!(
                        "Earn a star on {} to open it.",
                        crate::ponds::pond(n - 1).name
                    )),
                );
            }
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Ponds, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Snowfall Skate title."),
        );
        items
    }
}
