//! The sound check: every hum and cue of the game, by name.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Bus, Ctx, Sound};
use parlor_core::{Parlor, ParlorAction};

use crate::rules::{Colour, Kind};
use crate::sound;

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    /// The sounds playing now, stopped when another plays or on leaving.
    playing: Vec<Sound>,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new("Sound check").with_intro_help(
                "Up and Down choose a sound, Enter plays it and says what it means, Escape goes back.",
            ),
            playing: Vec::new(),
        }
    }

    fn hush(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in self.playing.drain(..) {
            ctx.audio.stop(s, 100);
        }
    }

    fn hear(&mut self, ctx: &mut Ctx<Parlor>, sounds: &[Sound]) {
        self.hush(ctx);
        self.playing.extend_from_slice(sounds);
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        Self::new()
    }
}

fn label(meaning: &str) -> String {
    meaning
        .split(':')
        .next()
        .unwrap_or(meaning)
        .trim_end_matches('.')
        .to_string()
}

impl Menu for SoundCheck {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "cupids_quiver_sounds"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        hearken::menu::menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.hush(ctx);
        sound::title_music(ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for colour in Colour::ALL {
            let name = sentence_start(&Kind::Heart(colour).name());
            let meaning = format!(
                "{name}: a soft paper hum, {}. Both hearts of a {} pair hum it.",
                colour.note_words(),
                colour.name()
            );
            items.push(
                MenuItem::new(name, move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    let a = sound::play(ctx, sound::HUM);
                    ctx.audio.set_pitch(a, colour.pitch());
                    let b = sound::play(ctx, sound::CHIME);
                    ctx.audio.set_pitch(b, colour.pitch());
                    s.hear(ctx, &[a, b]);
                    ctx.say(meaning.clone());
                })
                .select_sound(None)
                .help("A heart's hum, then the chime it makes when hit."),
            );
        }
        for (key, meaning) in sound::catalog() {
            items.push(
                MenuItem::new(
                    label(meaning),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let a = if key.contains("/music/") || key.contains("/ambience/") {
                            ctx.audio.play(key, Bus::MUSIC)
                        } else {
                            sound::play(ctx, key)
                        };
                        s.hear(ctx, &[a]);
                        ctx.say(meaning);
                    },
                )
                .select_sound(None)
                .help("Enter plays it and says what it means."),
            );
        }
        let enter_key = format!(
            "Enter key: between rooftops, Enter climbs to the next one, the same as {}.",
            ctx.control_name(ParlorAction::Primary)
        );
        items.push(
            MenuItem::new(
                "Enter between rooftops",
                move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    sound::play(ctx, sound::ROUND_START);
                    ctx.say(enter_key.clone());
                },
            )
            .select_sound(None)
            .help("The one fixed key of the game, beyond the shared controls."),
        );
        items.push(
            MenuItem::new("Back", |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Cupid's Quiver title."),
        );
        items
    }
}
