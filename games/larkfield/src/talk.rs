//! What settlers say: at the gate, when you stop for a word, and at
//! supper. Lines are built from who they are, what they want and where
//! their story has got to, so a chat always tells you something.

use parlor_core::Rng;

use crate::people::{Needs, Trait, WANTS};
use crate::rules::{Hamlet, Mood, Settler, Wobble};

/// The stranger at the gate, asking to stay.
pub fn arrival(s: &Settler) -> String {
    format!(
        "{name}, once a {trade}. {name}: {hook} Good at {best}. {a} and {b}.",
        name = s.name,
        trade = s.trade.name(),
        hook = s.trade.story().hook,
        best = s.trade.skills()[0].name(),
        a = crate::rules::sentence_case(s.traits[0].name()),
        b = s.traits[1].name(),
    )
}

/// Talking a wobbling settler round.
pub fn talked_round(s: &Settler) -> String {
    let n = &s.name;
    match s.wobble {
        Some(Wobble::Bridge) => format!("You sit with {n} on the bridge for a while. {n} feels better, and goes back to work."),
        Some(Wobble::Scones) => format!("You eat one of {n}'s scones and say it is the best yet. {n} laughs, and goes back to work."),
        Some(Wobble::SameSong) => format!("You ask {n} for a different tune. {n} plays it, smiles, and goes back to work."),
        _ => format!("You talk with {n} for a while. {n} feels better."),
    }
}

/// What a story step waits for, in the settler's words.
fn story_wish(needs: Needs) -> String {
    match needs {
        Needs::Building(b) => format!("If only we had {}.", b.name()),
        Needs::Goods(id, n) => format!("I need {} for it.", crate::buildings::count(id, n)),
        Needs::Settlers(n) => format!("Once there are {n} of us, I can start."),
        Needs::Festival => "The next festival is the time for it.".to_string(),
        Needs::Cheerful => "I just need a good day.".to_string(),
    }
}

/// A word with a settler: their mood first, then one thing worth knowing.
pub fn chat(s: &Settler, rng: &mut Rng) -> String {
    let n = &s.name;
    let mood = s.mood();
    let opener = match mood {
        Mood::Cheerful => format!("{n} is cheerful"),
        Mood::Fine => format!("{n} is fine"),
        Mood::Glum => format!("{n} is glum"),
    };
    let reasons = s.reasons();
    let why = match reasons.first() {
        Some(r) => format!("{opener}: {r}."),
        None => format!("{opener}."),
    };
    let mut options: Vec<String> = Vec::new();
    if let Some(w) = s.want {
        let (what, _) = WANTS[w % WANTS.len()];
        options.push(format!("{n}: I'd love {what}."));
    }
    if s.story_step < 3 {
        let story = s.trade.story();
        let (needs, _) = story.steps[s.story_step as usize];
        let line = if s.story_step == 0 {
            format!("{n}: {} {}", story.hook, story_wish(needs))
        } else {
            format!("{n}: {}", story_wish(needs))
        };
        options.push(line);
    }
    if s.has(Trait::Homesick) {
        options.push(format!("{n}: A letter from home would be lovely."));
    }
    if s.has(Trait::Chatterbox) {
        options.push(format!("{n} tells you all about the weather, twice."));
    }
    if s.company < 0.3 {
        options.push(format!("{n}: It's been a lonely day."));
    }
    if options.is_empty() {
        options.push(format!("{n} is {}.", s.doing()));
    }
    let pick = rng.pick(&options).clone();
    format!("{why} {pick}")
}

/// The talk at supper: a page or two about the people at the table.
pub fn supper_talk(h: &Hamlet, seat: Option<&str>, rng: &mut Rng) -> Vec<String> {
    let mut pages = Vec::new();
    let here: Vec<&Settler> = h.settlers.iter().filter(|s| !s.away()).collect();
    if here.is_empty() {
        return pages;
    }
    if let Some(name) = seat {
        if let Some(s) = here.iter().find(|s| s.name == name) {
            let line = match s.mood() {
                Mood::Glum => format!("You sit with {name}, who is glad of the company."),
                _ => {
                    let bit = match s.story_step {
                        0 => s.trade.story().hook.to_string(),
                        1 | 2 => "Things are coming along, aren't they?".to_string(),
                        _ => "I never thought I'd find a home like this.".to_string(),
                    };
                    format!("You sit with {name}. {name}: {bit}")
                }
            };
            pages.push(line);
        }
    }
    // A line or two about the table, different most nights.
    let mut lines: Vec<String> = Vec::new();
    for s in &here {
        for (other, &v) in &s.liking {
            if s.name < *other && here.iter().any(|o| &o.name == other) {
                let (a, b) = (&s.name, other);
                if v >= 5 {
                    lines.push(
                        rng.pick(&[
                            format!(
                                "{a} and {b} laugh at the same joke. They are good friends now."
                            ),
                            format!("{a} saves {b} a seat. They are good friends."),
                            format!("{a} and {b} wash up together, still talking."),
                        ])
                        .clone(),
                    );
                } else if v <= -3 {
                    lines.push(format!(
                        "{a} and {b} bicker over the last potato. They will come round."
                    ));
                }
            }
        }
    }
    let low = h.food_days() < 2;
    for s in &here {
        let n = &s.name;
        let line = match s.traits[rng.below(2) as usize] {
            Trait::Chatterbox => format!("{n} talks all through supper."),
            Trait::Hungry => format!("{n} has seconds, and then thirds."),
            Trait::SweetTooth if h.has("pie") > 0 => format!("{n} saves the pie for last."),
            Trait::NightOwl => format!("{n} stays up after everyone, watching the river."),
            Trait::EarlyRiser => format!("{n} is first to bed, as always."),
            Trait::Homesick => format!("{n} talks about home for a while."),
            Trait::Grumbler => format!("{n} grumbles about the soup, then has more."),
            Trait::Worrier if low => format!("{n} counts the jars in the larder twice."),
            Trait::Clumsy => format!("{n} knocks over a jug. Nobody minds."),
            Trait::Brave => format!("{n} tells a story about a bear. It gets bigger every time."),
            _ => continue,
        };
        lines.push(line);
    }
    for _ in 0..2 {
        if lines.is_empty() {
            break;
        }
        let i = rng.below(lines.len() as u32) as usize;
        pages.push(lines.remove(i));
    }
    if h.is_built(crate::buildings::Building::Bandstand) && here.len() > 1 {
        pages.push("After supper there is music on the bandstand.".to_string());
    } else if let Some(m) = here.iter().find(|s| s.has(Trait::Musician)) {
        if rng.chance(0.5) {
            pages.push(format!(
                "{} plays the {} after supper.",
                m.name,
                m.instrument()
            ));
        }
    }
    if pages.is_empty() {
        let s = here[rng.below(here.len() as u32) as usize];
        pages.push(format!("{} is {} tonight.", s.name, s.mood().name()));
    }
    if h.music_night {
        pages.push(
            "It is music night. A band from the next village plays on the green till late."
                .to_string(),
        );
    }
    pages
}
