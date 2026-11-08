//! The sound check: every bell and cue of the game, by name.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound};
use parlor_core::Parlor;

use crate::rules::{bell_words, VILLAGE};
use crate::sound;

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    /// The sound playing now, stopped when another plays or on leaving, so
    /// a long one never piles up under the next.
    playing: Vec<Sound>,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new("Sound check")
                .with_intro_help("Up and Down choose a sound, Enter plays it, Escape goes back."),
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
        "belfry_sounds"
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

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for b in 1..=VILLAGE.bells {
            let words = bell_words(b, VILLAGE.bells);
            items.push(
                MenuItem::new(words, move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.stop_speech();
                    let a = sound::play(ctx, sound::bell(b));
                    s.hear(ctx, &[a]);
                })
                .select_sound(None)
                .help("One of the six village bells, rung alone."),
            );
        }
        items.push(
            MenuItem::new("Cathedral bells", |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.stop_speech();
                let a = sound::play(ctx, sound::bell(7));
                let b = sound::play(ctx, sound::bell(8));
                s.hear(ctx, &[a, b]);
            })
            .select_sound(None)
            .help("The two deepest bells, which only the cathedral peal uses."),
        );
        for (key, meaning) in sound::catalog() {
            items.push(
                MenuItem::new(
                    label(meaning),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.stop_speech();
                        let a = if key.contains("/music/") || key.contains("/ambience/") {
                            ctx.audio.play(key, Bus::MUSIC)
                        } else {
                            sound::play(ctx, key)
                        };
                        s.hear(ctx, &[a]);
                    },
                )
                .select_sound(None)
                .help("Enter plays it."),
            );
        }
        items.push(
            MenuItem::new("Back", |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Belfry title."),
        );
        items
    }
}
