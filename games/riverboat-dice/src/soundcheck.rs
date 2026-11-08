//! The sound check: every sound of the game, by name.

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
    /// The seat the seats row played last, from 1.
    seat: usize,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new("Sound check").with_intro_help(
                "Up and Down choose a sound, Enter plays it and says what it means, Escape goes back.",
            ),
            playing: Vec::new(),
            seat: 0,
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
        "riverboat_dice_sounds"
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
            items.push(
                MenuItem::new(
                    label(meaning),
                    move |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        let a = if key == sound::TITLE_MUSIC {
                            ctx.audio.play(key, Bus::MUSIC)
                        } else if key == sound::PADDLE_WHEEL
                            || key == sound::HIGH_WATER
                            || key == sound::SALOON_BAND
                        {
                            ctx.audio.play(key, Bus::AMBIENCE)
                        } else {
                            sound::play(ctx, key)
                        };
                        s.hear(ctx, &[a]);
                        ctx.say(meaning);
                    },
                )
                .select_sound(None)
                .help("Enter plays it and says what it means."),
            );
        }
        items.push(
            MenuItem::new(
                "Seats around the table",
                |s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    s.seat = s.seat % 3 + 1;
                    let a = sound::chair(ctx, s.seat, 3);
                    s.hear(ctx, &[a]);
                    ctx.say(match s.seat {
                        1 => "The Widow's chair, on your left. Enter again for the next seat.",
                        2 => {
                            "The Gambler's chair, across the table. Enter again for the next seat."
                        }
                        _ => "The Captain's chair, on your right. Enter again for the first seat.",
                    });
                },
            )
            .select_sound(None)
            .help("Where each player sits, by the sound of their chair."),
        );
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help("Return to the Riverboat Dice title."),
        );
        items
    }
}
