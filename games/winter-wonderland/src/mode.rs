//! How a run was started, and the story that strings both parts together.

use crate::challenge::Plan;

/// How a run was started; decides where it goes when it ends.
#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    /// Chosen from the title: Gift Sort from a shift, or one night.
    Free,
    /// A daily, weekly or monthly challenge, the same for everyone.
    /// `gifts` carries the weekly's Gift Sort score into its night.
    Challenge { plan: Box<Plan>, gifts: u64 },
    /// Midnight Countdown with no way to fail and nothing saved.
    Practice,
    /// The story: three shifts, then a night. `gifts` carries the Gift
    /// Sort score into the countdown.
    Story { blizzard: bool, gifts: u64 },
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

    /// The night the story plays.
    pub fn story_night(blizzard: bool) -> u32 {
        if blizzard {
            3
        } else {
            1
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

/// The story's opening pages.
pub fn story_intro(blizzard: bool) -> Vec<String> {
    if blizzard {
        vec![
            "A blizzard has buried the North Pole, and the workshop is a day behind. The belt runs fast, the guests keep changing their minds, and the return sack is filling up with oddities.".into(),
            "Three hard shifts on the belt, then New Year's Eve in the village square, where the band plays faster and trickier than before.".into(),
        ]
    } else {
        vec![
            "The elves have wrapped every gift beautifully, and then mixed up every label.".into(),
            "Shake each gift as it rides the conveyor, hear what is inside, and send it to whoever wished for it before it falls off the end of the belt.".into(),
            "Three shifts stand between you and the morning. And a week later, a party will need a conductor.".into(),
        ]
    }
}

/// The bridge between the two halves.
pub fn story_bridge(blizzard: bool) -> Vec<String> {
    let party = crate::night::NightSpec::new(Mode::story_night(blizzard)).name;
    vec![
        "Every gift is delivered. The morning comes and goes in a happy blur of paper and ribbon.".into(),
        format!("A week later, the snow has settled and the village gathers at {party} for New Year's Eve. The band leader has lost her voice, and she hands you the baton."),
        "The band will play a rhythm, and you answer it. Bells first, then drums, then horns, all the way to midnight.".into(),
    ]
}

/// The story's last page, before the score.
pub fn story_ending() -> String {
    "The bells ring twelve. Corks pop, fireworks bloom over the snow, and the whole village sings. The last gift is delivered, and the party has made it to midnight. Happy New Year!".into()
}
