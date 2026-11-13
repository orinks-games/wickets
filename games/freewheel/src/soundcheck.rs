//! The sound check: every cue by name and meaning, and where sounds sit
//! around you.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound};
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::sound::{self, place};

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    /// What is playing now, stopped when another plays or on leaving.
    playing: Vec<Sound>,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new(labels::SOUND_CHECK)
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

/// Loops and music play a few seconds in the sound check.
fn is_loop(key: &str) -> bool {
    [
        "/roll",
        "/freehub",
        "/pedal",
        "/wind",
        "edge_rumble",
        "door_zone",
        "/traffic/car",
        "/idle",
        "/throttle",
        "/cobbles",
        "/ambience/",
        "/music/",
    ]
    .iter()
    .any(|k| key.contains(k))
        || key.ends_with("/traffic/oncoming")
}

impl Menu for SoundCheck {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "freewheel_sounds"
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

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = vec![
            MenuItem::new("Left and right", |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.stop_speech();
                let a = sound::play_at(ctx, sound::MARKER, place(3.0, 4.0));
                s.hear(ctx, &[a]);
            })
            .select_sound(None)
            .help("Plays a hazard tick placed to your right."),
            MenuItem::new("Behind you", |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.stop_speech();
                let a = ctx
                    .audio
                    .start_loop(sound::CAR, Some(place(-1.5, -8.0)), Bus::EFFECTS);
                s.hear(ctx, &[a]);
            })
            .select_sound(None)
            .help("Plays a car placed behind you."),
            MenuItem::new(
                "Nearer and faster",
                |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.stop_speech();
                    let mut all = Vec::new();
                    for (metres, pitch) in [(30.0, 1.0f32), (15.0, 1.25), (4.0, 1.45)] {
                        let a = sound::play_at(ctx, sound::MARKER, place(0.0, metres));
                        ctx.audio.set_pitch(a, pitch);
                        all.push(a);
                    }
                    s.hear(ctx, &all);
                },
            )
            .select_sound(None)
            .help("Plays a hazard tick rising as it comes near."),
        ];
        for (key, meaning) in sound::catalog() {
            items.push(
                MenuItem::new(
                    sound::label(meaning),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.stop_speech();
                        let a = if key.contains("/music/") || key.contains("/ambience/") {
                            ctx.audio.play(key, Bus::MUSIC)
                        } else if is_loop(key) {
                            ctx.audio.start_loop(key, None, Bus::EFFECTS)
                        } else {
                            sound::play(ctx, key)
                        };
                        s.hear(ctx, &[a]);
                    },
                )
                .select_sound(None)
                .help("Play this sound."),
            );
        }
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.stop_speech();
                ctx.pop()
            })
            .help("Return to the Freewheel title."),
        );
        items
    }
}
