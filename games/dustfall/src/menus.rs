//! The sound check: every cue a player has to know, played from where it
//! would be heard.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Vec3};
use parlor_core::menus::labels;
use parlor_core::Parlor;

use crate::rules::lanes::{HORIZON, MID, NEAR};
use crate::rules::{Kind, CENTRE};
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

/// A sound played from a place in the sky.
fn at(
    label: &str,
    key: &'static str,
    lane: u8,
    height: u32,
    help: &str,
) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |_s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        ctx.stop_speech();
        sound::at(ctx, key, lane, height);
    })
    .select_sound(None)
    .help(help)
}

/// A sound played from under a lane, at the dome's edge.
fn under(label: &str, key: &'static str, lane: u8, help: &str) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |_s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        ctx.stop_speech();
        sound::under(ctx, key, lane);
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
        // Rows alternate sides so left against right is heard.
        let (far_left, left, right, far_right) = (0, 1, 3, 4);
        vec![
            at("A drifter's engine", sound::engine(Kind::Drifter), left, MID, "A soft drone falling down its lane. Drifters fire a bolt at the gun's lane from mid range."),
            at("A skitter's engine", sound::engine(Kind::Skitter), right, MID, "A quick buzz. Skitters fall fast, come in pairs, and some weave a lane when the gun finds them."),
            at("A hulk's engine", sound::engine(Kind::Hulk), left, NEAR, "A heavy throb. Hulks fall slowly, take 3 hits, and siphon the shield from near range."),
            at("A sapper's engine", sound::engine(Kind::Sapper), right, NEAR, "A ratchet. Sappers head down the lane over your hydroponics and darken the building they breach over."),
            at("A mule's engine", sound::engine(Kind::Mule), far_left, MID, "A chugging engine crossing the sky at mid range. It never attacks: shoot it for its food and metal before it leaves."),
            at("The mothership", sound::engine(Kind::Mothership), CENTRE, MID, "A deep pulse. It holds at mid range, slides from lane to lane, drops drifters and winds up siege shots. Flak cannot hurt it, and the gun only bites once it is at mid range."),
            at("The carrier", sound::engine(Kind::Carrier), far_right, MID, "A vast hum crossing the sky at mid range on the last night, letting skitters down as it goes."),
            plain("A wave is called", sound::WAVE, "The call before a wave: its raiders come over the horizon a second later, one after another."),
            plain("A wave down", sound::WAVE_CLEAR, "The last raider of a wave is gone. The next wave is called after a short lull."),
            at("A raider arrives", sound::ARRIVE, far_left, HORIZON, "A ping at the horizon where a raider enters, or where a skitter is dropped."),
            at("A raider weaves", sound::WEAVE, right, MID, "A weaving raider slipping to the next lane, away from the gun."),
            at("A raider leaves", sound::LEAVE, far_right, MID, "A mule going off the far side of the sky with what it came for."),
            at("The scan", sound::SCAN, right, MID, "The blip of Up and Down scanning, heard where the raider is."),
            under("The gun slides", sound::GUN_RAIL, left, "A scrape along the rail, under the new lane."),
            under("The lock tone", sound::LOCK, right, "The pulse under the gun when a raider is in its lane. It pulses faster as the raider nears."),
            under("The gun fires", sound::GUN_FIRE, left, "The gun's shot, under its lane."),
            at("A hit", sound::GUN_HIT, right, MID, "A shot ringing on a raider that has hits left."),
            at("A raider down", sound::GUN_DOWN, left, NEAR, "The burst of a raider going down, where it was."),
            plain("A missile", sound::MISSILE, "A missile away from the dome: 4 damage to the nearest raider in any lane."),
            at("A raider charging", sound::CHARGE_WIND, right, MID, "A rising whine in the lane a raider is about to fire down. Slide the gun out of it."),
            plain("A shot dodged", sound::CHARGE_PASS, "The bolt passing: you left the lane in time. The same whoosh follows a gun shot that glances off a far mothership."),
            plain("A shot hits", sound::CHARGE_HIT, "The bolt landing on the shield or the dome."),
            under("A breach on the shield", sound::BREACH_SHIELD, left, "A raider reaching the dome hits the shield, under its lane."),
            under("A breach on the dome", sound::BREACH_DOME, right, "A breach that reaches the dome itself. At dome 0 the colony falls."),
            plain("A building goes dark", sound::BREACH_BUILDING, "The building under a breached lane stops working until repaired."),
            at("A mine goes off", sound::MINE, right, MID, "A mine under the first raider past mid range in its lane: 2 damage, and the mine is spent."),
            under("A flak post fires", sound::FLAK, left, "A powered post firing up its own lane."),
            at("The dust storm", sound::STORM_WIND, far_left, MID, "The storm's wind at the lane it covers, on the dust storm's nights. Nothing in that lane is heard, and the flak post under it is silent."),
            plain("Bought", sound::BUY, "Something built, laid or repaired in the colony phase."),
            plain("Too dear", sound::CANT, "The colony phase refusing: not enough in store."),
            plain("Night held", sound::HELD, "The last wave is down. The colony phase opens."),
            MenuItem::new(labels::BACK, |_s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Return to the Dustfall title."),
        ]
    }
}
