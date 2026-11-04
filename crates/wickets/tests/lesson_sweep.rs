//! Every tutorial, left alone and then mashed with play keys, must stay a
//! lesson or end back on its game's title: never fall through to a run's
//! results, a save, or the next screen of the real game.

use hearken::{Headless, Key};
use parlor_core::{Date, GameInfo, ParlorData};
use wickets::{CategoryList, GameList, Lobby};

/// Ten minutes of play, in tenths of a second.
const FRAMES: usize = 10 * 60 * 10;

/// `mash`: a seed for random key presses, or `None` to leave it alone.
fn sweep(g: &GameInfo, today: Date, mash: Option<u64>) -> Result<(), String> {
    let data = ParlorData {
        today: Some(today),
        ..Default::default()
    };
    let mut h = Headless::new(data, Lobby::new());
    h.app.ctx.push(CategoryList::new());
    h.app.ctx.push(GameList::new(g.category));
    h.frame(0.0);
    (g.open)(&mut h.app.ctx);
    h.frame(0.0);
    let title = h.app.top_name().to_string();
    let lesson = (g.tutorial)(&mut h.app.ctx);
    h.app.ctx.push(lesson);
    h.frame(0.0);
    let depth = h.app.depth();
    let keys = [
        Key::Left,
        Key::Right,
        Key::Up,
        Key::Down,
        Key::Space,
        Key::Return,
        Key::Tab,
        Key::Space,
        Key::Num1,
        Key::Num2,
    ];
    let mut seed = mash.unwrap_or(0);
    for i in 0..FRAMES {
        let top = h.app.top_name();
        if top == title && h.app.depth() < depth {
            return Ok(());
        }
        if top != "lesson" && h.app.depth() <= depth {
            return Err(format!(
                "{}: left the lesson for {top} after {} seconds{}: {:?}",
                g.id,
                i / 10,
                if mash.is_some() {
                    " of key presses"
                } else {
                    " alone"
                },
                h.spoken().iter().rev().take(2).collect::<Vec<_>>()
            ));
        }
        if mash.is_some() && i % 3 == 0 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            h.press(keys[(seed >> 33) as usize % keys.len()]);
        }
        h.frame(0.1);
    }
    Ok(())
}

#[test]
fn no_tutorial_falls_through_to_the_real_game() {
    let autumn = Date::new(2026, 10, 16);
    let mut games: Vec<(GameInfo, Date)> =
        wickets::games().into_iter().map(|g| (g, autumn)).collect();
    games.extend(wickets::story_games().into_iter().map(|(g, _)| (g, autumn)));
    for (g, holiday) in wickets::specials() {
        let day = holiday.date(2026).expect("the holiday falls in 2026");
        games.push((g, day));
    }
    let failures: Vec<String> = games
        .iter()
        .flat_map(|(g, day)| [None, Some(7), Some(99), Some(2024)].map(|m| sweep(g, *day, m)))
        .filter_map(Result::err)
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}
