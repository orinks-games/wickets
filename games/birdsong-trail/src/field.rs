//! The field guide: every bird met, with its song in words, a line about
//! it, and the songs learned.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::birds::{self, Kind, Species};
use crate::guide::{self, Entry, Guide};
use crate::sound::Tune;

/// The field guide's `State::name`.
pub const FIELD_NAME: &str = "birdsong_trail_guide";

/// The title row: "Field guide: 12 of 53 birds".
pub fn row_label(g: &Guide) -> String {
    format!("Field guide: {} of {} birds", g.birds(), birds::ALL.len())
}

/// What a bird's row reads: "Robin, 2 of 3 songs".
pub fn entry_label(species: &Species, e: &Entry) -> String {
    let name = sentence_start(species.name);
    match species.kind {
        Kind::Flock => format!("{name}, counted"),
        _ => match e.verses {
            0 => format!("{name}, heard"),
            1 => format!("{name}, 1 of {} songs", species.verses()),
            n => format!("{name}, {n} of {} songs", species.verses()),
        },
    }
}

/// The whole entry, as Enter reads it.
pub fn entry_text(species: &Species, e: &Entry) -> String {
    let met = match e.met {
        1 => "once".to_string(),
        2 => "twice".to_string(),
        n => format!("{n} times"),
    };
    let songs = match species.kind {
        Kind::Flock => "Counted on migration.".to_string(),
        _ => format!("Songs in your guide: {} of {}.", e.verses, species.verses()),
    };
    format!(
        "{}. Its song: {}. {} {} First met {}, met {met}.",
        sentence_start(species.name),
        species.sounds_like,
        songs,
        species.about,
        e.first
    )
}

/// Where the birds still to meet might be, for the last row's help.
pub fn still_to_meet(g: &Guide) -> String {
    let kinds = [Kind::Trail, Kind::Flock];
    let parts: Vec<String> = kinds
        .iter()
        .filter_map(|k| {
            let n = birds::ALL
                .iter()
                .filter(|s| s.kind == *k && !g.has(s))
                .count();
            (n > 0).then(|| match k {
                Kind::Trail => format!("{n} on the trails, some only in certain seasons"),
                Kind::Flock => format!("{n} in the migration flocks of the equinoxes"),
            })
        })
        .collect();
    if parts.is_empty() {
        "Every bird is in your guide.".to_string()
    } else {
        format!("Still to meet: {}.", parts.join(". "))
    }
}

pub struct FieldGuide {
    menu: MenuCore<Self, Parlor>,
    tune: Tune,
}

impl FieldGuide {
    pub fn new() -> FieldGuide {
        FieldGuide {
            menu: MenuCore::new("Field guide").with_intro_help(
                "Up and Down choose a bird, Enter plays its song and reads its page, Escape goes back.",
            ),
            tune: Tune::new(),
        }
    }
}

impl Default for FieldGuide {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for FieldGuide {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        FIELD_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.tune.stop(ctx);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        self.tune.tick(ctx, dt);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let g = Guide::load(ctx);
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for species in birds::ALL.iter().filter(|s| g.has(s)) {
            let e = g.entry(species).cloned().unwrap_or_default();
            let text = entry_text(species, &e);
            let help = format!("Its song: {}.", species.sounds_like);
            items.push(
                MenuItem::new(
                    entry_label(species, &e),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let notes = species.notes();
                        s.tune.song(ctx, species, &notes);
                        ctx.say(text.clone());
                    },
                )
                .select_sound(None)
                .help(sentence_start(&help)),
            );
        }
        let missing = g.missing();
        let label = match missing {
            0 => "Every bird met".to_string(),
            1 => "1 bird still to meet".to_string(),
            n => format!("{n} birds still to meet"),
        };
        let line = still_to_meet(&g);
        let songs = format!(
            "{} of {} songs in your guide. {line}",
            g.songs(),
            guide::total_songs()
        );
        let said = songs.clone();
        items.push(
            MenuItem::new(label, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.say(said.clone())
            })
            .select_sound(None)
            .help(songs),
        );
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to where you were."),
        );
        items
    }
}
