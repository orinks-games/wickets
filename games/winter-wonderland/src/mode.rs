//! How a run was started, and the story that strings Gift Sort and the
//! Wish Desk together.

use crate::challenge::Plan;

/// How a run was started; decides where it goes when it ends.
#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    /// Chosen from the title: Gift Sort or the Wish Desk from a shift or
    /// stack, or one night of Midnight Countdown.
    Free,
    /// A daily, weekly or monthly challenge, the same for everyone.
    /// `gifts` carries the weekly's Gift Sort score to the Wish Desk.
    Challenge { plan: Box<Plan>, gifts: u64 },
    /// Midnight Countdown with no way to fail and nothing saved.
    Practice,
    /// The story: three shifts, then a stack at the Wish Desk. `gifts`
    /// carries the Gift Sort score to the desk.
    Story { blizzard: bool, gifts: u64 },
    /// Muddled post: one stack of letters from the youngest children.
    Muddled,
}

impl Mode {
    /// The shifts a Gift Sort run in this mode plays, first to last.
    pub fn shifts(&self, start: u32) -> (u32, u32) {
        match self {
            Mode::Story {
                blizzard: false, ..
            } => (1, 3),
            Mode::Story { blizzard: true, .. } => (4, 6),
            Mode::Challenge { plan, .. } => plan.shifts(),
            _ => (start, crate::gifts::SHIFTS),
        }
    }

    /// The Wish Desk stacks a run in this mode plays, first to last.
    pub fn stacks(&self, start: u32) -> (u32, u32) {
        match self {
            Mode::Story { blizzard, .. } => {
                let s = Mode::story_stack(*blizzard);
                (s, s)
            }
            Mode::Challenge { plan, .. } => plan.stacks(),
            Mode::Muddled => (crate::events::MUDDLED_STACK, crate::events::MUDDLED_STACK),
            _ => (start, start),
        }
    }

    /// The Wish Desk stack the story plays.
    pub fn story_stack(blizzard: bool) -> u32 {
        if blizzard {
            4
        } else {
            2
        }
    }

    pub fn is_story(&self) -> bool {
        matches!(self, Mode::Story { .. })
    }

    /// The challenge plan, for a challenge run.
    pub fn plan(&self) -> Option<&Plan> {
        match self {
            Mode::Challenge { plan, .. } => Some(plan),
            _ => None,
        }
    }
}

/// How a letter's writer is named: "Milo, age 6, writes".
pub const WRITES: &str = "writes";

/// The story's opening pages.
pub fn story_intro(blizzard: bool) -> Vec<String> {
    if blizzard {
        vec![
            "A blizzard has buried the North Pole, and the workshop is a day behind. The belt runs fast, the guests keep changing their minds, and the return sack is filling up with oddities.".into(),
            "You have three hard shifts on the belt. Then come the snowed-in letters at the Wish Desk, which nobody else can answer.".into(),
        ]
    } else {
        vec![
            "The elves have wrapped every gift beautifully, and then mixed up every label.".into(),
            "Shake each gift as it rides the conveyor, hear what is inside, and send it to whoever wished for it before it falls off the end of the belt.".into(),
            "Three shifts stand between you and the morning. And the letters are piling up on the Wish Desk.".into(),
        ]
    }
}

/// The bridge between the belt and the desk.
pub fn story_bridge(blizzard: bool) -> Vec<String> {
    if blizzard {
        vec![
            "The last shift is done, and the belt groans to a stop.".into(),
            "Then the snowplough brings one more sack. It holds the letters that the blizzard delayed, every one of them still waiting for its gift.".into(),
            "The gifts are on the Wish Desk's shelves, but the labels blew off in the storm. Read each letter, shake the boxes, and wrap the one that answers it. There is no clock at the desk.".into(),
        ]
    } else {
        vec![
            "Every gift is delivered. The belt stops, and the workshop goes quiet.".into(),
            "The Wish Desk is still busy. A stack of letters came too late for the belt, and their gifts sit on the shelves with no labels at all.".into(),
            "Read each letter, shake the boxes until one sounds like the wish, and wrap it. There is no clock at the desk.".into(),
        ]
    }
}

/// The story's last page, before the score.
pub fn story_ending() -> String {
    "The last letter is answered, and the stamp comes down on the last parcel. Outside, snow settles on the workshop roof.".into()
}
