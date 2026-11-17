//! The sound check: every cue in the game, each with its name.

use hearken::menu::{menu_enter, Menu, MenuCore, MenuItem};
use hearken::{Bus, Ctx, Sound, Vec3};
use parlor_core::{Parlor, ParlorAction};

use crate::gifts::{Kind, Spot};
use crate::night::Inst;
use crate::sound::{self, spot_pos};

pub struct SoundCheck {
    menu: MenuCore<Self, Parlor>,
    playing: Sound,
}

impl SoundCheck {
    pub fn new() -> Self {
        SoundCheck {
            menu: MenuCore::new("Sound check").with_intro_help(
                "Up and Down move between sounds, Enter plays one, Escape goes back.",
            ),
            playing: Sound::NONE,
        }
    }
}

impl Default for SoundCheck {
    fn default() -> Self {
        Self::new()
    }
}

/// A sound's name, as its row says it.
fn gift_label(kind: Kind) -> &'static str {
    match kind {
        Kind::Rattle => "Gift that would rattle",
        Kind::Squish => "Gift that would squish",
        Kind::Clink => "Gift that would clink",
        Kind::Jingle => "Gift that would jingle",
        Kind::Crinkle => "Gift that would crinkle",
        Kind::Slosh => "Gift that would slosh",
        Kind::Tick => "Gift that would tick",
        Kind::Squeak => "Gift that would squeak",
        Kind::Thump => "Gift that would thump",
        Kind::Chime => "Gift that would chime",
    }
}

fn item(label: String, key: &'static str, pos: Option<Vec3>) -> MenuItem<SoundCheck, Parlor> {
    MenuItem::new(label, move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        ctx.stop_speech();
        ctx.audio.stop(s.playing, 100);
        s.playing = match pos {
            Some(p) => ctx.audio.play_at(key, p, Bus::EFFECTS),
            None => ctx.audio.play(key, Bus::EFFECTS),
        };
    })
    .select_sound(None)
    .help("Enter plays it.")
}

impl Menu for SoundCheck {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "winter_wonderland_sounds"
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
        ctx.audio.stop(self.playing, 100);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let n = |a| ctx.control_name(a);
        let mut items: Vec<MenuItem<Self, Parlor>> = Kind::ALL
            .iter()
            .map(|&k| {
                item(
                    gift_label(k).to_string(),
                    sound::gift(k),
                    Some(sound::GIFT_POS),
                )
            })
            .collect();
        let cues: Vec<(String, &'static str, Option<Vec3>)> = vec![
            ("Gift lands on the belt".into(), sound::ARRIVE, None),
            ("Right recipient".into(), sound::CORRECT, Some(spot_pos(Spot::Left))),
            ("A gift sliding to a recipient".into(), sound::SEND, Some(spot_pos(Spot::Right))),
            ("Wrong guess".into(), sound::WRONG, Some(spot_pos(Spot::Ahead))),
            ("Into the return sack, behind you".into(), sound::SACK, Some(spot_pos(Spot::Sack))),
            ("Near the edge".into(), sound::WARN, None),
            ("A gift falls off the belt and breaks".into(), sound::FALL, None),
            ("Clock running low".into(), sound::CLOCK, None),
            ("Change of plans".into(), sound::CHANGE, None),
            ("Nothing there".into(), sound::EMPTY, None),
            ("Shift complete".into(), sound::CLEAR, None),
            ("Wish Desk: a step along the shelf, heard where the box sits".into(), sound::DESK_STEP, Some(sound::shelf_pos(0, 5))),
            ("Wish Desk: the end of the shelf".into(), sound::DESK_END, None),
            ("Wish Desk: an empty place, where a gift was wrapped".into(), sound::DESK_EMPTY, None),
            ("Wish Desk: a shake of only straw".into(), sound::DESK_STRAW, None),
            ({ let pressed = n(ParlorAction::Primary); format!("Wish Desk: wrapping paper, while you hold {pressed}") }, sound::DESK_WRAP, None),
            ("Wish Desk: the wrong box, unwrapped and put back".into(), sound::DESK_WRONG, None),
            ({ let pressed = n(ParlorAction::Primary); format!("Midnight Countdown bells, answered with {pressed}") }, sound::inst(Inst::Bell), None),
            ({ let pressed = n(ParlorAction::Down); format!("Drum, answered with {pressed}") }, sound::inst(Inst::Drum), None),
            ({ let pressed = n(ParlorAction::Up); format!("Horn, answered with {pressed}") }, sound::inst(Inst::Horn), None),
            ("Clock tick, on every beat".into(), sound::TICK, None),
            ("Tock, on the first beat of each bar".into(), sound::TOCK, None),
            ("Missed note".into(), sound::MISS, None),
            ("Wrong instrument".into(), sound::OOPS, None),
            ("A press with nothing to answer".into(), sound::STRAY, None),
            ("Score multiplier up".into(), sound::COMBO, None),
            ("A call answered perfectly".into(), sound::CLEAN, None),
            ("Party energy low".into(), sound::LOW, None),
        ];
        items.extend(cues.into_iter().map(|(l, k, p)| item(l, k, p)));
        items.push(
            MenuItem::new("Back", |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Back to the title."),
        );
        items
    }
}
