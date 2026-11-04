//! The journal: every species with its blink and tone, the chords, the
//! the midsummer keepsakes.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx};
use parlor_core::menus::labels;
use parlor_core::{Pages, Parlor};

use crate::progress::Journal;
use crate::sound;
use crate::species::{Chord, Species};

/// The journal menu's `State::name`.
pub const JOURNAL_NAME: &str = "firefly_meadow_journal";

pub struct JournalMenu {
    menu: MenuCore<Self, Parlor>,
    /// Flashes of the species being played: when each lights, and its key.
    flashes: Vec<(f64, &'static str)>,
    clock: f64,
}

impl JournalMenu {
    pub fn new() -> JournalMenu {
        JournalMenu {
            menu: MenuCore::new("Journal").with_intro_help(
                "Up and Down choose a page of the journal. Enter on a firefly plays its blink and reads about it. Escape goes back.",
            ),
            flashes: Vec::new(),
            clock: 0.0,
        }
    }

    /// Plays two whole blinks of `species`, flash by flash.
    fn blink(&mut self, species: Species) {
        let p = species.pattern();
        let key = sound::glow(species);
        self.flashes.clear();
        for cycle in 0..2 {
            let base = self.clock + cycle as f64 * p.burst().max(0.6) + cycle as f64 * 0.9;
            for i in 0..p.flashes {
                self.flashes.push((base + p.flash_start(i), key));
            }
        }
    }
}

impl Default for JournalMenu {
    fn default() -> Self {
        Self::new()
    }
}

/// "notes C and G" from "Notes C and G", to follow a colon.
fn lower_first(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|f| f.to_lowercase().chain(chars).collect())
        .unwrap_or_default()
}

/// "The Summer Triad, made 3 times" and the recipes of those still to find.
pub fn chord_pages(journal: &Journal) -> Vec<String> {
    let found: Vec<String> = Chord::ALL
        .into_iter()
        .filter(|&c| journal.has_chord(c))
        .map(|c| {
            let n = journal.chords.get(c.slug()).copied().unwrap_or(0);
            let times = if n == 1 {
                "once".to_string()
            } else {
                format!("{n} times")
            };
            format!(
                "{}, made {times}: {}",
                hearken::text::sentence_start(c.name()),
                lower_first(c.recipe())
            )
        })
        .collect();
    let missing = Chord::ALL.len() - found.len();
    let mut first = format!("Chords: {} of {} found.", found.len(), Chord::ALL.len());
    if !found.is_empty() {
        first.push(' ');
        first.push_str(&found.join(" "));
    }
    let mut pages = vec![first];
    if missing > 0 {
        let hints: Vec<String> = Chord::ALL
            .into_iter()
            .filter(|&c| !journal.has_chord(c))
            .map(|c| {
                format!(
                    "{}: {}",
                    hearken::text::sentence_start(c.name()),
                    lower_first(c.recipe())
                )
            })
            .collect();
        pages.push(format!(
            "Still to find. Each firefly rings one note, and a jar set free rings its notes together. {}",
            hints.join(" ")
        ));
    }
    pages
}

/// The keepsakes, year by year.
pub fn keepsake_pages(journal: &Journal) -> Vec<String> {
    if journal.keepsakes.is_empty() {
        return vec![
            "Keepsakes: none yet. Each year, midsummer night gives one keepsake, on the day of the summer solstice in June.".to_string(),
        ];
    }
    let lines: Vec<String> = journal
        .keepsakes
        .iter()
        .map(|(year, k)| format!("{year}: {}, {} percent in step.", k.name, k.harmony))
        .collect();
    vec![format!("Keepsakes. {}", lines.join(" "))]
}

impl Menu for JournalMenu {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        JOURNAL_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn exit(&mut self, _ctx: &mut Ctx<Parlor>) {
        self.flashes.clear();
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let journal = Journal::load(ctx);
        let mut items = Vec::new();
        for (i, species) in Species::ALL.into_iter().enumerate() {
            if journal.found(species) {
                let count = journal.count(species);
                let label = if species == Species::MidsummerChorus {
                    format!("{}: seen on midsummer night", species.name())
                } else {
                    format!("{}: {count} caught", species.name())
                };
                let text = format!(
                    "{}: {}. {} Its note is {}.",
                    species.name(),
                    species.blink_words(),
                    species.about(),
                    species.note().name()
                );
                items.push(
                    MenuItem::new(label, move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        s.blink(species);
                        ctx.say(text.clone());
                    })
                    .select_sound(None)
                    .help(species.about()),
                );
            } else {
                let label = format!("Unknown firefly {}", i + 1);
                let text = format!("Not found yet. {}", species.where_words());
                items.push(
                    MenuItem::new(label, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        sound::play(ctx, sound::NOTHING);
                        ctx.say(text.clone());
                    })
                    .select_sound(None)
                    .help(species.where_words()),
                );
            }
        }
        let chords = journal.chords_found();
        items.push(
            MenuItem::new(
                format!("Chords: {chords} of {}", Chord::ALL.len()),
                |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    let journal = Journal::load(ctx);
                    ctx.push(Pages::new(chord_pages(&journal)));
                },
            )
            .help("The chords your jars have rung, and how to make the rest."),
        );
        items.push(
            MenuItem::new(
                format!("Keepsakes: {}", journal.keepsakes.len()),
                |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    let journal = Journal::load(ctx);
                    ctx.push(Pages::new(keepsake_pages(&journal)));
                },
            )
            .help("One keepsake from each year's midsummer night."),
        );
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Firefly Meadow title."),
        );
        items
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        self.clock += dt;
        let now = self.clock;
        let due: Vec<&'static str> = self
            .flashes
            .iter()
            .filter(|(t, _)| *t <= now)
            .map(|(_, k)| *k)
            .collect();
        self.flashes.retain(|(t, _)| *t > now);
        for key in due {
            ctx.audio.play(key, Bus::EFFECTS);
        }
    }
}
