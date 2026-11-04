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
                "Up and Down move through the game's sounds, Enter plays one and says what it means, Escape goes back.",
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
                    let bus = if key.contains("/music/") {
                        Bus::MUSIC
                    } else {
                        Bus::EFFECTS
                    };
                    ctx.audio.play(key, bus);
                    ctx.say(meaning.clone());
                })
                .select_sound(None)
            })
            .collect();
        items.push(
            MenuItem::new("Left and right", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                sound::play_at(ctx, sound::item(crate::rules::Item::Clock), -3.0, 0.0);
                sound::play_at(ctx, sound::item(crate::rules::Item::Bell), 3.0, 0.0);
                ctx.say("A clock ticks to your left, three steps west, and a bell rings to your right, three steps east. Things ahead are north of you. Things behind, to the south, sound duller.");
            })
            .select_sound(None)
            .help("A clock on your left and a bell on your right, to hear which way is west and east."),
        );
        items.push(
            MenuItem::new("Near and far", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                sound::play_at(ctx, sound::item(crate::rules::Item::Crystal), 0.0, 5.0);
                ctx.say("The crystal hums five steps ahead: faint. Things call out faster and louder as you get nearer.");
            })
            .select_sound(None)
            .help("A crystal five steps ahead, to hear how far off a thing sounds."),
        );
        items.push(
            MenuItem::new("I lists what you carry", |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.say("I is a key of this game's own, and Settings cannot change it. During a haunt it lists what you carry, with no sound.");
            })
            .select_sound(None)
            .help("The one key of this game's own: I lists what you carry."),
        );
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.pop()
            })
            .help(format!("Return to the {} title.", crate::NAME)),
        );
        items
    }
}
