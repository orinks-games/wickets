//! The sound check: every firefly's tone and every cue of the game, by name.

use hearken::menu::{menu_enter, Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::sound::{self, Play, CATALOG};
use crate::species::Species;

/// The sound check's `State::name`.
pub const SOUNDS_NAME: &str = "firefly_meadow_sounds";

/// Seconds a loop plays before it stops.
const LOOP_PREVIEW: f64 = 5.0;
/// Seconds of music before it stops.
const MUSIC_PREVIEW: f64 = 12.0;

enum Preview {
    None,
    Playing { sound: Sound, left: f64 },
    Sides { key: &'static str, wait: f64 },
}

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    preview: Preview,
}

impl SoundCheck {
    pub fn new() -> SoundCheck {
        SoundCheck {
            menu: MenuCore::new("Sound check").with_intro_help(
                "Up and Down move through the sounds you need in play. Enter plays the chosen sound. Escape goes back.",
            ),
            preview: Preview::None,
        }
    }

    fn stop(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Preview::Playing { sound, .. } = std::mem::replace(&mut self.preview, Preview::None)
        {
            ctx.audio.stop(sound, 300);
        }
    }

    fn play(&mut self, ctx: &mut Ctx<Parlor>, key: &'static str, how: Play, _text: String) {
        ctx.stop_speech();
        self.stop(ctx);
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        match how {
            Play::Once => {
                ctx.audio.play(key, Bus::EFFECTS);
            }
            Play::Sides => {
                ctx.audio
                    .play_at(key, Vec3::new(-4.0, 1.0, 0.0), Bus::EFFECTS);
                self.preview = Preview::Sides { key, wait: 1.6 };
            }
            Play::Loop => {
                let sound = ctx.audio.start_loop(key, None, Bus::AMBIENCE);
                self.preview = Preview::Playing {
                    sound,
                    left: LOOP_PREVIEW,
                };
            }
            Play::Music => {
                let sound = ctx.audio.start_loop(key, None, Bus::MUSIC);
                self.preview = Preview::Playing {
                    sound,
                    left: MUSIC_PREVIEW,
                };
            }
        }
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        Self::new()
    }
}

/// Every key the sound check plays, for tests.
pub fn all_keys() -> Vec<&'static str> {
    let mut keys: Vec<&'static str> = CATALOG.iter().map(|e| e.key).collect();
    for s in Species::ALL {
        keys.push(sound::glow(s));
        if s != Species::MidsummerChorus {
            keys.push(sound::bloom(s));
        }
    }
    keys.sort();
    keys.dedup();
    keys
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
        menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop(ctx);
        sound::title_music(ctx);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items = Vec::new();
        for species in Species::ALL {
            let text = format!(
                "{}: {}. Its note is {}. A firefly's tone comes from where it is in the meadow.",
                species.name(),
                species.blink_words(),
                species.note().name()
            );
            items.push(
                MenuItem::new(
                    format!("{}'s glow", species.name()),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.stop_speech();
                        s.play(ctx, sound::glow(species), Play::Sides, text.clone())
                    },
                )
                .select_sound(None)
                .help(format!("The tone of one {} flash.", species.name())),
            );
        }
        for entry in CATALOG {
            if entry.key == sound::BONFIRE {
                continue;
            }
            let text = format!("{}. {}", entry.name, entry.meaning);
            items.push(
                MenuItem::new(entry.name, move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.stop_speech();
                    s.play(ctx, entry.key, entry.play, text.clone())
                })
                .select_sound(None)
                .help(entry.meaning),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.stop_speech();
                ctx.pop()
            })
            .help("Return to the Firefly Meadow title."),
        );
        items
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        let mut done = false;
        match &mut self.preview {
            Preview::None => {}
            Preview::Playing { left, .. } => {
                *left -= dt;
                done = *left <= 0.0;
            }
            Preview::Sides { key, wait } => {
                *wait -= dt;
                if *wait <= 0.0 {
                    ctx.audio
                        .play_at(key, Vec3::new(4.0, 1.0, 0.0), Bus::EFFECTS);
                    done = true;
                }
            }
        }
        if done {
            self.stop(ctx);
        }
    }
}
