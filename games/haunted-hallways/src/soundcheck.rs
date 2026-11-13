//! The sound check: every cue of the game, by name.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx};
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::sound;

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new("Sound check").with_intro_help(
                "Up and Down move through the game's sounds, Enter plays one, Escape goes back.",
            ),
        }
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
        "haunted_hallways_sounds"
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

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = sound::catalog()
            .into_iter()
            .map(|(key, meaning)| {
                let label = meaning
                    .split(':')
                    .next()
                    .unwrap_or(&meaning)
                    .trim_end_matches('.')
                    .to_string();
                MenuItem::new(label, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.stop_speech();
                    let bus = if key.contains("/music/") {
                        Bus::MUSIC
                    } else {
                        Bus::EFFECTS
                    };
                    ctx.audio.play(key, bus);
                })
                .select_sound(None)
            })
            .collect();
        items.push(
            MenuItem::new("Left and right", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    ctx.stop_speech();
                sound::play_at(ctx, sound::item(crate::rules::Item::Clock), -3.0, 0.0);
                sound::play_at(ctx, sound::item(crate::rules::Item::Bell), 3.0, 0.0);
            })
            .select_sound(None)
            .help("A clock on your left and a bell on your right, to show which way is west and east."),
        );
        items.push(
            MenuItem::new("Near and far", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.stop_speech();
                sound::play_at(ctx, sound::item(crate::rules::Item::Crystal), 0.0, 5.0);
            })
            .select_sound(None)
            .help("A crystal five steps ahead, to show how far off a thing sounds."),
        );
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.stop_speech();
                ctx.pop()
            })
            .help(format!("Return to the {} title.", crate::NAME)),
        );
        items
    }
}
