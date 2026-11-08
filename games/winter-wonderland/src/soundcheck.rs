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

fn item(label: String, key: &'static str, pos: Option<Vec3>) -> MenuItem<SoundCheck, Parlor> {
    // Desk cues share a "Wish Desk: " prefix, so their names run to the first full stop.
    let label = match label.strip_prefix("Wish Desk: ") {
        Some(rest) => format!("Wish Desk: {}", rest.split('.').next().unwrap_or(rest)),
        None => label.split([':', '.']).next().unwrap_or(&label).to_string(),
    };
    MenuItem::new(label, move |s: &mut SoundCheck, ctx: &mut Ctx<Parlor>| {
        ctx.stop_speech();
        ctx.audio.stop(s.playing, 100);
        s.playing = match pos {
            Some(p) => ctx.audio.play_at(key, p, Bus::EFFECTS),
            None => ctx.audio.play(key, Bus::EFFECTS),
        };
    })
    .select_sound(None)
    .help("Press Enter to hear it.")
}

impl Menu for SoundCheck {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "sound_check"
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
                    format!(
                        "Gift that would {}: {}. Wishes like {}.",
                        k.name(),
                        k.describe(),
                        k.items().join(", ")
                    ),
                    sound::gift(k),
                    Some(sound::GIFT_POS),
                )
            })
            .collect();
        let cues: Vec<(String, &'static str, Option<Vec3>)> = vec![
            ("Gift lands on the belt: a soft thump and a rustle of paper.".into(), sound::ARRIVE, None),
            ("Right recipient: rising bells from their side. This one is on your left.".into(), sound::CORRECT, Some(spot_pos(Spot::Left))),
            ("A gift sliding to a recipient. This one goes right.".into(), sound::SEND, Some(spot_pos(Spot::Right))),
            ("Wrong guess: a low buzz from that side. It costs clock time. This one is straight ahead.".into(), sound::WRONG, Some(spot_pos(Spot::Ahead))),
            ("Into the return sack, behind you.".into(), sound::SACK, Some(spot_pos(Spot::Sack))),
            ("Near the edge: quick beeping. Send the gift fast.".into(), sound::WARN, None),
            ("A gift falls off the belt and breaks.".into(), sound::FALL, None),
            ("Clock running low: a tick each second near the end.".into(), sound::CLOCK, None),
            ("Change of plans: a doorbell, when someone changes their wish.".into(), sound::CHANGE, None),
            ("Nothing there: a dull knock.".into(), sound::EMPTY, None),
            ("Shift complete.".into(), sound::CLEAR, None),
            ("Wish Desk: a step along the shelf, heard where the box sits. This one is at the left end.".into(), sound::DESK_STEP, Some(sound::shelf_pos(0, 5))),
            ("Wish Desk: the end of the shelf.".into(), sound::DESK_END, None),
            ("Wish Desk: an empty place, where a gift was wrapped.".into(), sound::DESK_EMPTY, None),
            ("Wish Desk: a shake of only straw. Shake again to hear the gift.".into(), sound::DESK_STRAW, None),
            (format!("Wish Desk: wrapping paper, while you hold {}. Let go now and nothing is wrapped.", n(ParlorAction::Primary)), sound::DESK_WRAP, None),
            ("Wish Desk: the wrong box, unwrapped and put back.".into(), sound::DESK_WRONG, None),
            (format!("Midnight Countdown bells, answered with {}.", n(ParlorAction::Primary)), sound::inst(Inst::Bell), None),
            (format!("Drum, answered with {}.", n(ParlorAction::Down)), sound::inst(Inst::Drum), None),
            (format!("Horn, answered with {}.", n(ParlorAction::Up)), sound::inst(Inst::Horn), None),
            ("Clock tick, on every beat.".into(), sound::TICK, None),
            ("Tock, on the first beat of each bar.".into(), sound::TOCK, None),
            ("Missed note: a deflating party blower.".into(), sound::MISS, None),
            ("Wrong instrument: a short buzz.".into(), sound::OOPS, None),
            ("A press with nothing to answer: a soft tap.".into(), sound::STRAY, None),
            ("Score multiplier up: rising chimes.".into(), sound::COMBO, None),
            ("A call answered perfectly: sparkles.".into(), sound::CLEAN, None),
            ("Party energy low: a heartbeat.".into(), sound::LOW, None),
        ];
        items.extend(cues.into_iter().map(|(l, k, p)| item(l, k, p)));
        items.push(
            MenuItem::new("Back", |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.pop())
                .help("Back to the title."),
        );
        items
    }
}
