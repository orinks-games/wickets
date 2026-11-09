//! The sound check: every cue a player has to know, played from where it
//! would be heard.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Vec3};
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::rules::grid::CENTRE;
use crate::rules::{Kind, Sector};
use crate::sound;

pub const SOUNDS_NAME: &str = "dustfall_sounds";

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
}

impl SoundCheck {
    pub fn new() -> SoundCheck {
        SoundCheck {
            menu: MenuCore::new("Sound check").with_intro_help(
                "Up and Down choose a sound, Enter plays it from where you would hear it, Escape goes back.",
            ),
        }
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        SoundCheck::new()
    }
}

/// A sound played from a sector of the grid.
fn at(label: &str, key: &'static str, sector: Sector, help: &str) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |_s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        ctx.stop_speech();
        sound::at(ctx, key, sector);
    })
    .select_sound(None)
    .help(help)
}

/// A sound with no place.
fn plain(label: &str, key: &'static str, help: &str) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |_s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        ctx.stop_speech();
        ctx.audio.play(key, Bus::EFFECTS);
    })
    .select_sound(None)
    .help(help)
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
        ctx.audio.set_listener(Vec3::ZERO, 0.0);
        hearken::menu::menu_enter(self, ctx);
    }

    fn build_items(&mut self, _ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let far = Sector::new(CENTRE, 5);
        let near = Sector::new(CENTRE, 2);
        let right = Sector::new(3, 4);
        vec![
            at("A drifter steps", sound::raider(Kind::Drifter), right, "A soft whoosh at its new sector. Drifters fire a bolt at the gun's column from 3 rows out."),
            at("A skitter steps", sound::raider(Kind::Skitter), right, "Two quick ticks: skitters take 2 steps a phase and slip to the side the gun is not on."),
            at("A hulk steps", sound::raider(Kind::Hulk), right, "A heavy tread. Hulks rest every other phase, bash walls, and siphon the shield from 2 rows out."),
            at("A sapper steps", sound::raider(Kind::Sapper), right, "A ratchet. Sappers head for your hydroponics and darken the building they breach under."),
            at("A mule crosses", sound::raider(Kind::Mule), right, "A chugging engine crossing row 4. It never attacks: shoot it for its food and metal before it leaves."),
            at("The mothership", sound::raider(Kind::Mothership), Sector::new(CENTRE, 3), "A deep pulse. It holds at the centre of row 3, drops skitters and winds up siege shots."),
            at("A raider arrives", sound::ARRIVE, far, "A ping at the far row where a raider enters, or where a skitter is dropped."),
            plain("The gun slides", sound::GUN_RAIL, "A scrape along the rail, under the new column."),
            plain("The gun fires", sound::GUN_FIRE, "The gun's shot. A hit rings on the raider; a kill ends in a burst."),
            at("A raider down", sound::GUN_DOWN, near, "The burst of a raider going down, where it stood."),
            plain("A missile", sound::MISSILE, "A missile away: 4 damage to the nearest raider anywhere on the grid."),
            at("A raider charging", sound::CHARGE_WIND, Sector::new(CENTRE, 3), "A rising whine under the column a raider is about to fire at. Move the gun off it."),
            plain("A shot dodged", sound::CHARGE_PASS, "The bolt passing: you moved off the column in time."),
            plain("A shot hits", sound::CHARGE_HIT, "The bolt landing on the shield or the dome."),
            plain("A breach on the shield", sound::BREACH_SHIELD, "A raider at the dome's edge hits the shield."),
            plain("A breach on the dome", sound::BREACH_DOME, "A breach that reaches the dome itself. At dome 0 the colony falls."),
            plain("A building goes dark", sound::BREACH_BUILDING, "The building under a breached column stops working until repaired."),
            at("A wall is bashed", sound::WALL_CRACK, right, "A wall takes a bash. The second brings it down."),
            at("A wall falls", sound::WALL_DOWN, right, "The wall is gone and the sector is open."),
            at("A mine goes off", sound::MINE, right, "A mine under a raider: 2 damage, and the mine is spent."),
            at("A flak post fires", sound::FLAK, Sector::new(CENTRE, 3), "A powered post firing at the nearest raider beside it."),
            plain("Your turn", sound::TURN, "The chime that starts each of your turns."),
            plain("Turn over", sound::TURN_OVER, "Your turn ends and the raiders move."),
            plain("Bought", sound::BUY, "Something built, placed or repaired in the colony phase."),
            plain("Too dear", sound::CANT, "The colony phase refusing: not enough in store."),
            plain("Night held", sound::HELD, "The last raider is gone, or has left. The colony phase opens."),
            MenuItem::new(labels::BACK, |_s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Dustfall title."),
        ]
    }
}
