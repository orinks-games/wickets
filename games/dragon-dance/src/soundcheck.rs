//! The sound check: every cue of the game, by name, with what it means.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::Parlor;

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
}

impl Default for SoundCheck {
    fn default() -> Self {
        Self::new()
    }
}

/// The part of a meaning before its colon: the row's label.
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
        "dragon_dance_sounds"
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        hearken::menu::menu_enter(self, ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.hush(ctx);
        sound::title_music(ctx);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = Vec::new();
        for (lane, place) in [
            (0, "on the left"),
            (1, "in the middle"),
            (2, "on the right"),
        ] {
            items.push(
                MenuItem::new(
                    format!("Pearl {place}"),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        s.hush(ctx);
                        let a = sound::play_at(ctx, sound::PEARL_MOVE, sound::pearl_pos(lane, 3));
                        s.playing.push(a);
                        let find = ctx.control_name(parlor_core::ParlorAction::Secondary);
                        ctx.say(format!(
                            "The pearl's chime from lane {} of 3, {place}. In play {find} rings it and names its lane.",
                            lane + 1
                        ));
                    },
                )
                .select_sound(None)
                .help("The pearl's chime from one lane of a 3-lane street."),
            );
        }
        for (key, meaning) in sound::catalog() {
            items.push(
                MenuItem::new(
                    label(meaning),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        s.hush(ctx);
                        let loops = key.contains("/music/")
                            || key.contains("/ambience/")
                            || key.ends_with("/loop");
                        let a = if loops {
                            let bus = if key.contains("/music/") {
                                Bus::MUSIC
                            } else {
                                Bus::EFFECTS
                            };
                            ctx.audio.play(key, bus)
                        } else {
                            sound::play(ctx, key)
                        };
                        s.playing.push(a);
                        ctx.say(meaning);
                    },
                )
                .select_sound(None)
                .help("Enter plays it and says what it means."),
            );
        }
        items.push(
            MenuItem::new("Back", |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Dragon Dance title."),
        );
        items
    }
}
