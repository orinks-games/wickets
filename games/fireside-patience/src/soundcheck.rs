//! Sound check: gameplay cues by name, with playback.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound};
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::sound;

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    /// The sounds playing now, stopped when another plays or on leaving,
    /// so a long one never piles up under the next.
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
}

impl Default for SoundCheck {
    fn default() -> Self {
        Self::new()
    }
}

/// The row label: the name before the colon of its meaning.
pub fn label(meaning: &str) -> String {
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
        "fireside_patience_sounds"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.set_bed(None);
        hearken::menu::menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.hush(ctx);
        sound::fireside(ctx);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for (key, meaning) in sound::catalog() {
            items.push(
                MenuItem::new(
                    label(meaning),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.stop_speech();
                        s.hush(ctx);
                        let a = if sound::is_bed(key) {
                            ctx.audio.play(key, Bus::AMBIENCE)
                        } else {
                            sound::play(ctx, key)
                        };
                        s.playing.push(a);
                    },
                )
                .select_sound(None)
                .help("Enter plays it."),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Fireside Patience title."),
        );
        items
    }
}
