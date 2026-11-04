//! The calendar events: king tides in winter and moonlit pools on full
//! moons, each a title row of its own, locked while it is closed.

use crate::common::{self, choose, played, read_pages, spoke, Twin, SEED, TODAY};

use hearken::{Headless, Key};
use parlor_core::almanac;
use parlor_core::menus;
use parlor_core::{Date, Parlor, Records};
use tide_pools::events::{self, Event};
use tide_pools::outing::{outing_for, OutingScreen};
use tide_pools::rules::{self, Mode, Place, Species, Tide, TimeOfDay, KING_TIDE_EXTRA};
use tide_pools::{sound, ID, INFO};

/// The full moon of 26 October 2026, the Hunter's Moon.
const HUNTERS_MOON: Date = Date::new(2026, 10, 26);

/// The first king tide day of the 2026 winter: the first new or full
/// moon night from 1 November.
fn king_tide_day() -> Date {
    almanac::next_day(Date::new(2026, 11, 1), 40, |d| Event::KingTide.is_open(d))
        .unwrap()
        .0
}

/// The game's title on `date`.
fn title_on(date: Date) -> Headless<Parlor> {
    let mut h = common::base();
    h.app.ctx.game.today = Some(date);
    h.app.ctx.push(tide_pools::title());
    h.frame(0.0);
    h.clear();
    h
}

fn rows(h: &Headless<Parlor>) -> Vec<String> {
    menus::rows_of(&h.app.top_lines())
}

#[test]
fn the_events_open_on_their_nights_only() {
    assert!(events::open_on(TODAY).is_empty(), "the tests' quiet day");
    assert_eq!(events::open_on(HUNTERS_MOON), [Event::Moonlit]);
    let king = king_tide_day();
    assert!(Event::KingTide.is_open(king));
    assert!(king.month == 11 && king.day < 14, "{king:?}");
    // A summer full moon is moonlit, but never a king tide.
    let (summer, _) = almanac::next_full_moon(Date::new(2027, 7, 1));
    assert_eq!(events::open_on(summer), [Event::Moonlit]);
    // Every winter month has a king tide, and no summer month does.
    let days = |y: i32, m: u32| (1..=28).map(move |d| Date::new(y, m, d));
    for (y, m) in [(2026, 11), (2026, 12), (2027, 1), (2027, 2)] {
        assert!(days(y, m).any(|d| Event::KingTide.is_open(d)), "{y}-{m}");
    }
    for m in 3..=10 {
        assert!(!days(2027, m).any(|d| Event::KingTide.is_open(d)), "{m}");
    }
    // A moonlit night comes at least every 31 days, all year.
    let mut day = Date::new(2027, 1, 1);
    for _ in 0..12 {
        let (next, gap) = almanac::next_full_moon(day);
        assert!(gap <= 31, "{day:?}");
        assert!(Event::Moonlit.is_open(next));
        day = Date::from_days(next.days() + 3);
    }
}

#[test]
fn the_launcher_names_whichever_event_is_on() {
    let event = INFO.event.unwrap();
    assert!(event(TODAY).is_none());
    let moon = event(HUNTERS_MOON).unwrap();
    assert_eq!(moon.title, "Moonlit pools");
    assert!(moon.line.contains("the Hunter's Moon"), "{}", moon.line);
    let king = event(king_tide_day()).unwrap();
    assert_eq!(king.title, "King tide");
    assert!(king.line.contains("two extra pools"));
}

#[test]
fn event_rows_are_open_only_while_open_and_locked_in_place_otherwise() {
    let quiet = rows(&title_on(TODAY));
    assert_eq!(quiet[1], "King tide, locked", "{quiet:?}");
    assert_eq!(quiet[2], "Moonlit pools, locked", "{quiet:?}");
    let moon = rows(&title_on(HUNTERS_MOON));
    assert_eq!(moon[1], "King tide, locked", "{moon:?}");
    assert_eq!(moon[2], "Moonlit pools, Sandy Cove", "{moon:?}");
    let king = rows(&title_on(king_tide_day()));
    assert_eq!(king[1], "King tide, Sandy Cove", "{king:?}");
    let after = Date::from_days(HUNTERS_MOON.days() + 3);
    let after = rows(&title_on(after));
    assert_eq!(after[2], "Moonlit pools, locked", "{after:?}");
    assert_eq!(quiet.len(), moon.len());
    assert_eq!(quiet.len(), king.len());
}

#[test]
fn a_locked_event_row_says_when_it_opens() {
    let day = Date::new(2026, 10, 29);
    let king = Event::KingTide.locked_line(day);
    assert!(king.starts_with("King tide opens "), "{king}");
    assert!(king.contains("November"), "{king}");
    let mut h = title_on(day);
    choose(&mut h, "Moonlit pools, locked");
    assert!(
        spoke(&h, &Event::Moonlit.locked_line(day)),
        "{:?}",
        h.spoken()
    );
    assert!(Event::Moonlit
        .locked_line(day)
        .starts_with("Moonlit pools open "));
    assert_eq!(h.app.top_name(), tide_pools::TITLE_NAME);
}

#[test]
fn an_event_trip_goes_to_the_furthest_shore_you_have_opened() {
    let mut h = title_on(HUNTERS_MOON);
    let mut tank = tide_pools::Aquarium::default();
    tank.add(
        &[
            Species::Crab,
            Species::Periwinkle,
            Species::Starfish,
            Species::Anemone,
        ],
        "a test",
    );
    tank.store(&mut h.app.ctx);
    h.app.ctx.pop();
    h.app.ctx.push(tide_pools::title());
    h.frame(0.0);
    assert!(rows(&h).iter().any(|r| r == "Moonlit pools, Rocky Point"));
}

#[test]
fn a_king_tide_adds_two_pools_at_each_end_and_curlews() {
    let mut h = title_on(king_tide_day());
    choose(&mut h, "King tide");
    assert!(
        spoke(&h, "Sandy Cove. King tide. The sea is far out today"),
        "{:?}",
        h.spoken()
    );
    rules::fix_seed(Some(SEED));
    read_pages(&mut h);
    assert_eq!(h.app.top_name(), "tide_pools_outing");
    assert!(spoke(&h, "Pool 1 of 10"), "{:?}", h.spoken());
    rules::fix_seed(Some(SEED));
    let o = outing_for(Mode::Event(Event::KingTide, Place::Cove));
    assert_eq!(o.pools.len(), Place::Cove.pools() + 2 * KING_TIDE_EXTRA);
    assert_eq!(o.plan.conditions.tide, Tide::Spring);
    for p in [0, 1, 8, 9] {
        assert!(!o.pools[p].finds.is_empty(), "pool {p} is bare");
    }
    let mut t = Twin::new(h, o);
    t.h.clear();
    for _ in 0..30 {
        t.frame(0.5);
    }
    assert!(played(&t.h, sound::CURLEW));
    assert!(spoke(&t.h, "A curlew"));
    t.press(Key::S);
    t.press(Key::S);
    t.press(Key::S);
    assert!(spoke(&t.h, "King tide: pools 1, 2"));
}

#[test]
fn a_king_tide_starts_the_waves_further_out() {
    rules::fix_seed(Some(SEED));
    let king = OutingScreen::new(outing_for(Mode::Event(Event::KingTide, Place::Cove)), None);
    rules::fix_seed(Some(SEED));
    let plain = OutingScreen::new(outing_for(Mode::Beach(Place::Cove)), None);
    assert!(king.waves_pos().y > plain.waves_pos().y + 5.0);
    assert!(king.waves_volume() < plain.waves_volume());
}

#[test]
fn moonlit_pools_are_a_night_trip_with_a_heron_and_no_gulls() {
    let mut h = title_on(HUNTERS_MOON);
    choose(&mut h, "Moonlit pools");
    assert!(spoke(&h, "night under the full moon"), "{:?}", h.spoken());
    rules::fix_seed(Some(SEED));
    read_pages(&mut h);
    assert!(spoke(&h, "The full moon lights the pools."));
    rules::fix_seed(Some(SEED));
    let o = outing_for(Mode::Event(Event::Moonlit, Place::Cove));
    assert_eq!(o.plan.conditions.time, TimeOfDay::Night);
    assert!(o.plan.conditions.allows(Species::Jelly));
    assert!(o.plan.conditions.allows(Species::Octopus));
    assert!(!o.plan.conditions.allows(Species::Seahorse));
    let mut t = Twin::new(h, o);
    t.h.clear();
    for _ in 0..80 {
        t.frame(0.5);
    }
    assert!(played(&t.h, sound::HERON));
    assert!(!played(&t.h, sound::GULL));
    assert!(spoke(&t.h, "A night heron"));
}

#[test]
fn event_trips_score_on_their_own_boards() {
    let mut h = title_on(HUNTERS_MOON);
    choose(&mut h, "Moonlit pools");
    rules::fix_seed(Some(SEED));
    read_pages(&mut h);
    rules::fix_seed(Some(SEED));
    let mut t = Twin::new(h, outing_for(Mode::Event(Event::Moonlit, Place::Cove)));
    t.wait_for_tide();
    t.h.press(Key::Return);
    assert!(spoke(&t.h, "Your first score on the moonlit pools."));
    read_pages(&mut t.h);
    assert_eq!(Records::best(&mut t.h.app.ctx, ID, "moonlit"), Some(0));
    assert_eq!(Records::best(&mut t.h.app.ctx, ID, "cove"), None);
}

/// Finishes a moonlit trip on the Hunter's Moon, to its results menu.
fn moonlit_results() -> Twin {
    let mut h = title_on(HUNTERS_MOON);
    choose(&mut h, "Moonlit pools");
    rules::fix_seed(Some(SEED));
    read_pages(&mut h);
    rules::fix_seed(Some(SEED));
    let mut t = Twin::new(h, outing_for(Mode::Event(Event::Moonlit, Place::Cove)));
    t.wait_for_tide();
    t.h.press(Key::Return);
    read_pages(&mut t.h);
    assert_eq!(t.h.app.top_name(), "results");
    t
}

#[test]
fn play_again_after_the_moon_wanes_goes_back_to_the_title() {
    let mut t = moonlit_results();
    let after = Date::from_days(HUNTERS_MOON.days() + 3);
    assert!(!Event::Moonlit.is_open(after));
    t.h.app.ctx.game.today = Some(after);
    t.h.clear();
    choose(&mut t.h, "Play again");
    t.h.frame(0.0);
    assert_eq!(t.h.app.top_name(), tide_pools::TITLE_NAME);
    assert!(
        spoke(&t.h, "The moonlit pools have ended."),
        "{:?}",
        t.h.spoken()
    );
    assert!(rows(&t.h).iter().any(|r| r == "Moonlit pools, locked"));
    assert_eq!(Records::best(&mut t.h.app.ctx, ID, "moonlit"), Some(0));
}

#[test]
fn play_again_while_the_moon_is_full_sets_off_again() {
    let mut t = moonlit_results();
    t.h.clear();
    choose(&mut t.h, "Play again");
    t.h.frame(0.0);
    assert_ne!(t.h.app.top_name(), tide_pools::TITLE_NAME);
    assert!(
        spoke(&t.h, "night under the full moon"),
        "{:?}",
        t.h.spoken()
    );
}

#[test]
fn the_sound_check_leaves_out_the_seaside_atmosphere() {
    // The sound check lists only what a player needs to recognise in play.
    let names: Vec<String> = sound::catalog().into_iter().map(|(_, n, _)| n).collect();
    assert!(!names.iter().any(|n| n == "Curlew"));
    assert!(!names.iter().any(|n| n == "Night heron"));
}
