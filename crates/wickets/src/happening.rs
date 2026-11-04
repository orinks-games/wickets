//! Neutral stubs for optional sitting events.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Ctx, UiSound};
use parlor_core::menus::labels;
use parlor_core::story::PartOfDay;
use parlor_core::{Date, GameInfo, Parlor};
use serde::{Deserialize, Serialize};

use crate::games;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedHappening {
    pub kind: String,
    pub game: String,
    pub asker: String,
    pub text: String,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub misses: u32,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub missed: bool,
}

impl SavedHappening {
    pub fn is_riddle(&self) -> bool {
        self.kind == "riddle"
    }

    pub fn over(&self) -> bool {
        self.done || self.missed
    }
}

pub struct Ask<'a> {
    pub date: Date,
    pub part: PartOfDay,
    pub scenes: &'a [String],
    pub lit: &'a [String],
    pub hall: &'a [String],
    pub lamps: usize,
    pub post: bool,
}

pub fn plan(_ask: &Ask<'_>) -> Option<SavedHappening> {
    None
}

pub fn what(_h: &SavedHappening) -> String {
    "an event".to_string()
}

pub fn state(h: &SavedHappening) -> &'static str {
    if h.done {
        ": done"
    } else if h.missed {
        ": missed"
    } else {
        ""
    }
}

pub fn label(h: &SavedHappening) -> String {
    format!("Happening: {}{}", what(h), state(h))
}

pub fn stamp_line(stamps: u32, _met_host: bool) -> String {
    let book = "the parlor's stamp book";
    match stamps {
        1 => format!("A stamp for {book}. Your first stamp."),
        n => format!("A stamp for {book}. {n} stamps."),
    }
}

pub enum Guess {
    Again(String),
    Over(String),
}

type Judge = dyn FnMut(&mut Ctx<Parlor>, &str) -> Guess;

pub struct Riddle {
    menu: MenuCore<Self, Parlor>,
    answer: Box<Judge>,
    options: Vec<String>,
}

impl Riddle {
    pub fn new(
        h: &SavedHappening,
        answer: impl FnMut(&mut Ctx<Parlor>, &str) -> Guess + 'static,
    ) -> Self {
        Riddle {
            menu: MenuCore::new("The Echo's riddle").with_intro_help(
                "Up and Down choose a game, Enter names it. Listen again replays the tune.",
            ),
            answer: Box::new(answer),
            options: h.options.clone(),
        }
    }
}

impl Menu for Riddle {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "riddle"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items = Vec::new();
        for id in self.options.clone() {
            let g: Option<GameInfo> = games::all().into_iter().find(|g| g.id == id);
            let label = g.map(|g| g.name).unwrap_or("a game");
            items.push(
                MenuItem::new(
                    label,
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| match (s.answer)(ctx, &id) {
                        Guess::Again(line) => ctx.say(line),
                        Guess::Over(line) => ctx.pop_with(line),
                    },
                )
                .help(format!("Name {label}.")),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(UiSound::Back);
                ctx.pop()
            })
            .help("Return."),
        );
        items
    }
}
