//! The words of the tea house: what guests say as they come in, how they
//! ask, and how the special guests say goodbye. Plain functions, so tests
//! can read them.

use hearken::text::sentence_start;

use crate::days::Occasion;
use crate::guests::Regular;
use crate::rules::{Visit, Who};
use crate::teas::Tea;

/// What is said as a guest reaches the counter, with their request.
pub fn arrival(v: &Visit, _regulars: &[&'static Regular]) -> String {
    let ask = ask_words(v);
    match v.who {
        Who::Regular(r) if v.first => format!(
            "A new face: {}, {}, comes in. {ask}",
            r.name, r.who
        ),
        Who::Regular(r) => format!("{} comes in. {ask}", r.name),
        Who::WalkIn(w) => format!("{} {ask}", w.who),
        Who::Taster(t) => format!("{} {ask}", t.who),
        Who::Holiday(o) => holiday_arrival(o, v.year, v.wants),
        Who::Myself => ask,
    }
}

/// "Mister Hale: Something strong, please."
pub fn ask_words(v: &Visit) -> String {
    match v.who {
        Who::Myself => v.asks.clone(),
        _ => format!("{}: {}", v.name(), v.asks),
    }
}

/// What the guest would like, for the status: "something strong" or
/// "Sencha".
pub fn want_words(v: &Visit) -> String {
    match (v.wants, v.taste) {
        (Some(t), _) => t.name.to_string(),
        (None, Some(t)) => format!("something {}", t.word()),
        _ => "any tea you like".to_string(),
    }
}

/// "Mister Hale, June and Ada".
pub fn names(regulars: &[&'static Regular]) -> String {
    let names: Vec<&str> = regulars.iter().map(|r| r.name).collect();
    match names.as_slice() {
        [] => "your regulars".to_string(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// The day's special guest, coming in.
pub fn holiday_arrival(o: Occasion, year: i32, tea: Option<&Tea>) -> String {
    let tea = tea.map(|t| t.name).unwrap_or(o.tea().name);
    let animal = parlor_core::calendar::zodiac(year);
    match o {
        Occasion::LunarNewYear => format!(
            "Mei from the bakery comes in with a bag of tangerines and a tin of oolong. Mei: Happy New Year! It is the year of the {animal}. Will you brew this for me? {tea} is on your shelf today."
        ),
        Occasion::Valentines => format!(
            "An old couple come in arm in arm. Rosa: Fifty years married this spring. Rose tea for two, please, one pot and two cups. {tea} is on your shelf today."
        ),
        Occasion::SpringEquinox => format!(
            "A gardener comes in with soil on her sleeves. The gardener: First day of spring, and the cherry trees are out. Have you any cherry blossom tea? {tea} is on your shelf today."
        ),
        Occasion::Midsummer => format!(
            "A boy runs in with a kite under his arm, his mother just behind. The boy: It is the longest day of the year. Can I have something cold? {tea} is on your shelf today, steeped short and poured over ice."
        ),
        Occasion::AutumnEquinox => format!(
            "A farmer from the valley sets a sack down by the door. The farmer: Equinox today, day and night the same length. I brought barley, roasted this morning. {tea} is on your shelf today."
        ),
        Occasion::Halloween => format!(
            "The bell rings, but no footsteps follow. A figure in a long cloak stands at the counter, hood up. The stranger: Something smoky, if you please. Dark as tonight. {tea} is on your shelf today."
        ),
        Occasion::Thanksgiving => format!(
            "A grandmother comes in carrying a pie wrapped in a tea towel. The grandmother: The family is still on the road, and I am early. Spiced apple would go with this pie. {tea} is on your shelf today."
        ),
    }
}

/// What the day's guest leaves, after their cup.
pub fn farewell(o: Occasion, year: i32, _regulars: &[&'static Regular]) -> String {
    let keepsake = o.keepsake(year);
    match o {
        Occasion::LunarNewYear => format!(
            "Mei leaves you {keepsake}, and two tangerines for luck. Mei: May your kettle always sing."
        ),
        Occasion::Valentines => format!(
            "Rosa tucks {keepsake} beside your till. Bill: Same time next year, if you will have us."
        ),
        Occasion::SpringEquinox => format!(
            "The gardener leaves {keepsake} on the counter. The gardener: For your recipe book."
        ),
        Occasion::Midsummer => format!(
            "The boy leaves you {keepsake}, tied to the door handle, so it flutters when guests come in."
        ),
        Occasion::AutumnEquinox => format!(
            "The farmer leaves you {keepsake}, for the window. The farmer: For a good harvest."
        ),
        Occasion::Halloween => format!(
            "When you turn back from the shelf, the chair is empty. On the saucer lies {keepsake}, beside a live ember. Somewhere down the lane, someone cackles: Lantern Jack has paid for his tea."
        ),
        Occasion::Thanksgiving => format!(
            "The grandmother leaves you {keepsake}, and a slice of the pie. The grandmother: Thank you, dear."
        ),
    }
}

/// When to pour, as the kettle's song tells it: "Wait for the whistle."
pub fn heat_hint(tea: &Tea) -> String {
    let when = match tea.heat {
        0..=79 => "Pour during the small bubbles, before the hiss starts.",
        80..=91 => "Pour once the soft hiss starts, before it rolls.",
        92..=99 => "Pour as the hiss rolls, just before the whistle.",
        _ => "Wait for the whistle, then pour.",
    };
    format!("{} {when}", tea.needs())
}

/// The guest's words about the cup: "Mister Hale: That is lovely."
pub fn said(name: &str, words: &str) -> String {
    format!("{}: {words}", sentence_start(name))
}

/// "1 cup" or "3 cups".
pub fn cups_words(n: usize) -> String {
    match n {
        1 => "1 cup".to_string(),
        n => format!("{n} cups"),
    }
}

/// "1 second" or "5 seconds".
pub fn seconds_words(n: u32) -> String {
    match n {
        1 => "1 second".to_string(),
        n => format!("{n} seconds"),
    }
}

/// "1 coin" or "5 coins".
pub fn coins_words(n: u32) -> String {
    match n {
        1 => "1 coin".to_string(),
        n => format!("{n} coins"),
    }
}

/// "first", "second" ... for pouring a pot.
pub fn ordinal(n: u8) -> &'static str {
    match n {
        1 => "First",
        2 => "Second",
        3 => "Third",
        4 => "Fourth",
        5 => "Fifth",
        6 => "Sixth",
        7 => "Seventh",
        _ => "Next",
    }
}
