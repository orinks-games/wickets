//! The sound check: every cue of the game, by name.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::{Bus, Ctx, Sound};
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::rules::{tone_pitch, SURVEY};
use crate::sound;

/// The sound check's `State::name`.
pub const SOUNDS_NAME: &str = "soft_landing_sounds";

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    /// The sound playing now, stopped when another plays or on leaving, so
    /// a loop never runs on under the next.
    playing: Option<Sound>,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new(labels::SOUND_CHECK).with_intro_help(
                "Up and Down choose a sound, Enter plays it, Escape goes back.",
            ),
            playing: None,
        }
    }

    fn hear(&mut self, ctx: &mut Ctx<Parlor>, sound: Sound) {
        if let Some(old) = self.playing.replace(sound) {
            ctx.audio.stop(old, 100);
        }
    }

    fn hush(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(s) = self.playing.take() {
            ctx.audio.stop(s, 100);
        }
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

/// The descent tone at a falling speed, for the two tone rows.
fn tone_row(name: &'static str, speed: f64, meaning: &'static str) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(name, move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        let tone = sound::start_loop(ctx, sound::DESCENT_TONE, sound::TONE_VOLUME);
        ctx.audio.set_pitch(tone, tone_pitch(speed, SURVEY[0].safe));
        s.hear(ctx, tone);
        ctx.stop_speech();
    })
    .select_sound(None)
    .help(sentence_start(
        meaning.split(": ").nth(1).unwrap_or(meaning),
    ))
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
        let mut items: Vec<MenuItem<Self, Parlor>> = vec![
            tone_row(
                "Descent tone, safe",
                2.0,
                "Descent tone, safe: a low, steady tone. You are falling slower than the safe speed.",
            ),
            tone_row(
                "Descent tone, too fast",
                9.0,
                "Descent tone, too fast: the same tone, higher. The faster you fall, the higher it goes. You hear Too fast as it crosses the safe speed, and Safe speed as it comes back.",
            ),
        ];
        for (key, meaning) in sound::catalog() {
            items.push(
                MenuItem::new(
                    label(meaning),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let played = if key == sound::TITLE_MUSIC || key == sound::AMBIENCE {
                            ctx.audio.play(key, Bus::MUSIC)
                        } else if key == sound::ENGINE {
                            sound::start_loop(ctx, key, sound::ENGINE_VOLUME)
                        } else {
                            sound::play(ctx, key)
                        };
                        s.hear(ctx, played);
                        ctx.stop_speech();
                    },
                )
                .select_sound(None)
                .help(sentence_start(
                    meaning.split(": ").nth(1).unwrap_or(meaning),
                )),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                s.hush(ctx);
                ctx.pop()
            })
            .help("Return to the Soft Landing title."),
        );
        items
    }
}
