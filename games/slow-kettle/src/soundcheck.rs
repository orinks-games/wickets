//! Gameplay sound previews, including the cups.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound};
use parlor_core::Parlor;

use crate::sound;
use crate::teas::CUPS;

/// The sound check's `State::name`.
pub const SOUNDS_NAME: &str = "slow_kettle_sounds";

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    /// The sound playing now, stopped when another plays or on leaving.
    playing: Vec<Sound>,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new("Sound check").with_intro_help(
                "Up and Down choose a sound, Enter plays it, Escape goes back.",
            ),
            playing: Vec::new(),
        }
    }

    fn hush(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in self.playing.drain(..) {
            ctx.audio.stop(s, 100);
        }
    }

    fn hear(&mut self, ctx: &mut Ctx<Parlor>, key: &str, bus: Bus) {
        self.hush(ctx);
        let s = ctx.audio.play(key, bus);
        self.playing.push(s);
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
        SOUNDS_NAME
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
        for (key, meaning) in sound::catalog() {
            let bus = if key.contains("/music/") || key.contains("/ambience/") {
                Bus::MUSIC
            } else {
                Bus::EFFECTS
            };
            items.push(
                MenuItem::new(
                    label(meaning),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        s.hear(ctx, key, bus);
                        ctx.stop_speech();
                    },
                )
                .select_sound(None)
                .help("Enter plays it."),
            );
        }
        for cup in CUPS.iter() {
            items.push(
                MenuItem::new(cup.name, move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    s.hear(ctx, sound::cup(cup), Bus::EFFECTS);
                    ctx.stop_speech();
                })
                .select_sound(None)
                .help("One of the cups, set down on the counter."),
            );
        }
        items.push(
            MenuItem::new("Back", |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Slow Kettle title."),
        );
        items
    }
}
