//! The keys every launching screen reads the same way.

use hearken::{Ctx, InputEvent, Key};
use parlor_core::{Parlor, ParlorAction};

use crate::rules::{Kind, Pos};

/// The firework a number key launches: 1 boom, 2 crackle, 3 whistle,
/// 4 sparkle, on the top row or the keypad.
pub fn kind_for(event: &InputEvent) -> Option<Kind> {
    match event.pressed()? {
        Key::Num1 | Key::Kp1 => Some(Kind::Boom),
        Key::Num2 | Key::Kp2 => Some(Kind::Crackle),
        Key::Num3 | Key::Kp3 => Some(Kind::Whistle),
        Key::Num4 | Key::Kp4 => Some(Kind::Sparkle),
        _ => None,
    }
}

/// Where a launch goes now: left while the Left control or A is held,
/// right while Right or D is, else the center.
pub fn held_pos(ctx: &Ctx<Parlor>) -> Pos {
    if ctx.action_held(ParlorAction::Left) || ctx.held.is_pressed(Key::A) {
        Pos::Left
    } else if ctx.action_held(ParlorAction::Right) || ctx.held.is_pressed(Key::D) {
        Pos::Right
    } else {
        Pos::Center
    }
}

/// A press of an aiming control: Left or A, Right or D.
pub fn aim_for(ctx: &Ctx<Parlor>, event: &InputEvent) -> Option<Pos> {
    match ctx.action_for(event) {
        Some(ParlorAction::Left) => return Some(Pos::Left),
        Some(ParlorAction::Right) => return Some(Pos::Right),
        _ => {}
    }
    match event.pressed()? {
        Key::A => Some(Pos::Left),
        Key::D => Some(Pos::Right),
        _ => None,
    }
}

/// The keyboard's Enter. A controller's A also reads as Enter to menus,
/// but in play it is the main action, so only a real key counts here.
pub fn is_enter(event: &InputEvent) -> bool {
    matches!(event.key_down(), Some((Key::Return | Key::KpEnter, _, _)))
}

/// What the main action does while launching: a firework, or (on a show)
/// taking one back or finishing. Up and Down step through them, so a
/// controller, or anyone without number keys, can play every part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Fire(Kind),
    TakeBack,
    Finish,
}

impl Pick {
    /// "Crackle, key 2.", "Take back the last firework.", "Finish the show."
    pub fn spoken(self) -> String {
        match self {
            Pick::Fire(kind) => format!(
                "{}, key {}.",
                hearken::text::sentence_start(kind.name()),
                kind.digit()
            ),
            Pick::TakeBack => "Take back the last firework.".to_string(),
            Pick::Finish => "Finish the show and score it.".to_string(),
        }
    }
}

/// Every pick of a show or freestyle: the four fireworks, take back, finish.
pub const SHOW_PICKS: [Pick; 6] = [
    Pick::Fire(Kind::Boom),
    Pick::Fire(Kind::Crackle),
    Pick::Fire(Kind::Whistle),
    Pick::Fire(Kind::Sparkle),
    Pick::TakeBack,
    Pick::Finish,
];

/// The launch range's picks: just the fireworks.
pub const RANGE_PICKS: [Pick; 4] = [
    Pick::Fire(Kind::Boom),
    Pick::Fire(Kind::Crackle),
    Pick::Fire(Kind::Whistle),
    Pick::Fire(Kind::Sparkle),
];

/// The pick the main action uses, stepped with Up and Down (wrapping).
#[derive(Clone, Copy, Debug)]
pub struct Wheel {
    picks: &'static [Pick],
    at: usize,
}

impl Wheel {
    pub fn new(picks: &'static [Pick]) -> Wheel {
        Wheel { picks, at: 0 }
    }

    pub fn current(&self) -> Pick {
        self.picks[self.at]
    }

    /// Moves one pick on (`delta` > 0) or back, and returns the new one.
    pub fn step(&mut self, delta: i32) -> Pick {
        let n = self.picks.len() as i32;
        self.at = (self.at as i32 + delta.signum()).rem_euclid(n) as usize;
        self.current()
    }

    /// Selects a firework directly, as its number key does.
    pub fn select(&mut self, kind: Kind) {
        if let Some(i) = self.picks.iter().position(|p| *p == Pick::Fire(kind)) {
            self.at = i;
        }
    }
}

/// Up or Down, as a step for the wheel.
pub fn wheel_step(ctx: &Ctx<Parlor>, event: &InputEvent) -> Option<i32> {
    match ctx.action_for(event) {
        Some(ParlorAction::Down) => Some(1),
        Some(ParlorAction::Up) => Some(-1),
        _ => None,
    }
}

/// "Up and Down choose what the A button does: a firework, take back, or
/// finish."
pub fn wheel_line(ctx: &Ctx<Parlor>, finish: bool) -> String {
    let what = if finish {
        "a firework, take back, or finish"
    } else {
        "which firework"
    };
    format!(
        "{} and {} choose what {} does: {what}.",
        hearken::text::sentence_start(&ctx.control_name(ParlorAction::Up)),
        ctx.control_name(ParlorAction::Down),
        ctx.control_name(ParlorAction::Primary)
    )
}

/// "Hold the Left arrow or A for the left, the Right arrow or D for the
/// right."
pub fn aim_line(ctx: &Ctx<Parlor>) -> String {
    if on_pad(ctx) {
        return format!(
            "Hold {} to launch on the left, {} for the right; otherwise it goes up in the center.",
            ctx.control_name(ParlorAction::Left),
            ctx.control_name(ParlorAction::Right)
        );
    }
    format!(
        "Hold {} or A to launch on the left, {} or D for the right; otherwise it goes up in the center.",
        ctx.control_name(ParlorAction::Left),
        ctx.control_name(ParlorAction::Right)
    )
}

/// Whether the player is on a controller, so prompts name its buttons.
pub fn on_pad(ctx: &Ctx<Parlor>) -> bool {
    ctx.controller.device() == hearken::InputDevice::Controller
}

/// How to launch, for the device in use: the number keys, or the wheel.
pub fn keys_line(ctx: &Ctx<Parlor>) -> String {
    if on_pad(ctx) {
        format!(
            "{} launches the chosen firework. {}",
            hearken::text::sentence_start(&ctx.control_name(ParlorAction::Primary)),
            wheel_line(ctx, true)
        )
    } else {
        KEYS_LINE.to_string()
    }
}

/// The four keys and what they launch.
pub const KEYS_LINE: &str =
    "Keys 1 through 4 launch fireworks: 1 boom, 2 crackle, 3 whistle, 4 sparkle.";
