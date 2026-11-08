//! The sound check: every call, step and cue of the game, by name.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Bus, Ctx, Sound};
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::sound;
use crate::tunes::Step;

/// The sound check's `State::name`.
pub const SOUNDS_NAME: &str = "clover_jig_sounds";

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    /// The sound playing now, stopped when another plays or on leaving.
    playing: Vec<Sound>,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new(labels::SOUND_CHECK).with_intro_help(
                "Up and Down choose a sound, Enter plays it and says what it means, Escape goes back.",
            ),
            playing: Vec::new(),
        }
    }

    fn hear(&mut self, ctx: &mut Ctx<Parlor>, sounds: Vec<Sound>) {
        for old in self.playing.drain(..) {
            ctx.audio.stop(old, 100);
        }
        self.playing = sounds;
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        Self::new()
    }
}

/// What a step's row says: the call, the step's sound, and its key.
pub fn step_line(ctx: &Ctx<Parlor>, step: Step) -> String {
    format!(
        "{}: the caller says {}, then {}. {} dances it.",
        step.word(),
        step.lower(),
        step.sound_words(),
        sentence_start(&ctx.control_name(step.action()))
    )
}

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
        for s in self.playing.drain(..) {
            ctx.audio.stop(s, 100);
        }
        sound::title_music(ctx);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for step in Step::ALL {
            items.push(
                MenuItem::new(step.word(), move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    let a = sound::play_call(ctx, step);
                    let b = sound::play_step(ctx, step, 1.0);
                    s.hear(ctx, vec![a, b]);
                    let line = step_line(ctx, step);
                    ctx.say(line);
                })
                .select_sound(None)
                .help(format!(
                    "The {} step: the caller's word, then {}.",
                    step.lower(),
                    step.sound_words()
                )),
            );
        }
        for (key, meaning) in sound::catalog() {
            items.push(
                MenuItem::new(
                    label(meaning),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let played = if sound::is_music(key) {
                            ctx.audio.play(key, Bus::MUSIC)
                        } else {
                            sound::play(ctx, key)
                        };
                        s.hear(ctx, vec![played]);
                        ctx.say(meaning);
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
            .help("Return to the Clover Jig title."),
        );
        items
    }
}
