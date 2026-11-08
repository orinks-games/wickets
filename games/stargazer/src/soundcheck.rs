//! The sound check: every sound in the game, by name, with what it means.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound};
use parlor_core::Parlor;

use crate::sound::{self, Play};

/// The sound check's `State::name`.
pub const SOUNDS_NAME: &str = "stargazer_sounds";

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    /// What is playing now, stopped when another plays or on leaving.
    playing: Vec<Sound>,
    /// The second half of a two-part play: the right side, the higher
    /// note, or the rest of a sweep.
    pending: Option<Pending>,
}

struct Pending {
    key: String,
    sound: Option<Sound>,
    play: Play,
    t: f64,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new("Sound check").with_intro_help(
                "Up and Down choose a sound, Enter plays it, Escape goes back.",
            ),
            playing: Vec::new(),
            pending: None,
        }
    }

    fn hush(&mut self, ctx: &mut Ctx<Parlor>) {
        for s in self.playing.drain(..) {
            ctx.audio.stop(s, 100);
        }
        self.pending = None;
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        Self::new()
    }
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
        ctx.audio.set_listener(hearken::Vec3::ZERO, 0.0);
        hearken::menu::menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.hush(ctx);
        sound::title_music(ctx);
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let Some(p) = self.pending.as_mut() else {
            return;
        };
        p.t += dt;
        match p.play {
            Play::Sides if p.t >= 0.9 => {
                let s = sound::play_from(ctx, &p.key, 1.0, 0.0, 0.9, 1.0);
                self.playing.push(s);
                self.pending = None;
            }
            Play::Height if p.t >= 0.9 => {
                let s = sound::play_from(ctx, &p.key, 0.0, 1.0, 0.9, 1.26);
                self.playing.push(s);
                self.pending = None;
            }
            Play::Sweep => {
                if let Some(s) = p.sound {
                    let pan = (-1.0 + 2.0 * p.t / 1.2).min(1.0) as f32;
                    ctx.audio
                        .set_position(s, Some(hearken::Vec3::new(pan, 0.5, 0.0)));
                }
                if p.t >= 1.2 {
                    self.pending = None;
                }
            }
            _ => {}
        }
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for entry in sound::catalog() {
            let key = entry.key.clone();
            let play = entry.play;
            items.push(
                MenuItem::new(
                    entry.name.as_str(),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        s.hush(ctx);
                        let first = match play {
                            Play::Once => sound::play(ctx, &key),
                            Play::Music => ctx.audio.play(&key, Bus::MUSIC),
                            Play::Sides => sound::play_from(ctx, &key, -1.0, 0.0, 0.9, 1.0),
                            Play::Height => sound::play_from(ctx, &key, 0.0, 1.0, 0.9, 0.8),
                            Play::Sweep => sound::play_from(ctx, &key, -1.0, 0.5, 0.9, 1.0),
                        };
                        s.playing.push(first);
                        if matches!(play, Play::Sides | Play::Height | Play::Sweep) {
                            s.pending = Some(Pending {
                                key: key.clone(),
                                sound: Some(first),
                                play,
                                t: 0.0,
                            });
                        }
                        ctx.stop_speech();
                    },
                )
                .select_sound(None)
                .help("Enter plays it."),
            );
        }
        items.push(
            MenuItem::new("Back", |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Stargazer title."),
        );
        items
    }
}
