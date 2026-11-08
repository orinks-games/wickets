//! The sound check: every sound of the game, by name.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Bus, Ctx, Sound};
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::sound;

/// The sound check's `State::name`.
pub const SOUNDS_NAME: &str = "midnight_lanes_sounds";

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    /// The sound playing now, stopped when another plays or on leaving.
    playing: Option<Sound>,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new(labels::SOUND_CHECK)
                .with_intro_help("Up and Down choose a sound, Enter plays it, Escape goes back."),
            playing: None,
        }
    }

    fn hear(&mut self, ctx: &mut Ctx<Parlor>, sound: Sound) {
        if let Some(old) = self.playing.replace(sound) {
            ctx.audio.stop(old, 100);
        }
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        Self::new()
    }
}

/// A row's label: the name before the colon.
fn label(meaning: &str) -> String {
    meaning.split(':').next().unwrap_or(meaning).to_string()
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
        if let Some(s) = self.playing.take() {
            ctx.audio.stop(s, 100);
        }
        sound::title_music(ctx);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for (key, meaning) in sound::catalog() {
            items.push(
                MenuItem::new(
                    label(meaning),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.stop_speech();
                        let played = if key == sound::TITLE_MUSIC || key == sound::AMBIENCE {
                            ctx.audio.play(key, Bus::MUSIC)
                        } else {
                            sound::play(ctx, key)
                        };
                        s.hear(ctx, played);
                    },
                )
                .select_sound(None)
                .help(sentence_start(
                    meaning.split(": ").nth(1).unwrap_or(meaning),
                )),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Midnight Lanes title."),
        );
        items
    }
}
