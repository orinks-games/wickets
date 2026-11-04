//! Street select: every street of the main parade, open or locked.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Ctx, UiSound};
use parlor_core::menus::labels;
use parlor_core::{Parlor, Records};

use crate::dance::{self, stars_words, Launch, Mode};
use crate::progress::Progress;
use crate::streets::STREETS;
use crate::{sound, ID};

/// The menu's `State::name`.
pub const SELECT_NAME: &str = "dragon_dance_streets";

pub struct StreetSelect {
    menu: MenuCore<Self, Parlor>,
}

impl StreetSelect {
    pub fn new() -> Self {
        StreetSelect {
            menu: MenuCore::new("Street select").with_intro_help(
                "Up and Down choose a street, Enter dances it, Escape goes back. A street opens when the one before has a star.",
            ),
        }
    }
}

impl Default for StreetSelect {
    fn default() -> Self {
        Self::new()
    }
}

/// "Street 3: Silk Street, 2 stars" or "Street 5: Temple Road, locked".
pub fn row_label(progress: &Progress, n: usize) -> String {
    let base = format!("Street {n}: {}", STREETS[n - 1].name);
    if !progress.opened(n) {
        labels::locked(&base)
    } else if progress.played(n) {
        format!("{base}, {}", stars_words(progress.stars_on(n)))
    } else {
        format!("{base}, new")
    }
}

/// A street's help: what it is and the best score.
pub fn row_help(ctx: &mut Ctx<Parlor>, progress: &Progress, n: usize) -> String {
    let street = STREETS[n - 1];
    if !progress.opened(n) {
        return format!(
            "Earn a star on street {} to open it. {}.",
            n - 1,
            sentence_start(&street.describe())
        );
    }
    let best = match Records::best(ctx, ID, &format!("street {n}")) {
        Some(best) => format!("Best {best}."),
        None => "Not danced yet.".to_string(),
    };
    format!("{}. {best}", sentence_start(&street.describe()))
}

impl Menu for StreetSelect {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        SELECT_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let progress = Progress::load(ctx);
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for n in 1..=STREETS.len() {
            let help = row_help(ctx, &progress, n);
            let open = progress.opened(n);
            let why = help.clone();
            items.push(
                MenuItem::new(
                    row_label(&progress, n),
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        if open {
                            dance::start(ctx, Launch::new(Mode::Street(n)));
                        } else {
                            sound::play(ctx, sound::NOPE);
                            ctx.say(why.clone());
                        }
                    },
                )
                .help(help),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(UiSound::Back);
                ctx.pop()
            })
            .help("Return to the Dragon Dance title."),
        );
        items
    }
}
