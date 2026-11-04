//! Halloween on the title: the run-up floors that light candles, and the
//! Lantern Night, locked outside Halloween.

use crate::common::{goto, rows, Base, SEED};
use haunted_hallways::allhallows::{self, *};
use haunted_hallways::rules::{self, Mode};
use haunted_hallways::{Haunt, ID, INFO};
use hearken::{Headless, Key};
use parlor_core::{menus, Date, Parlor};

fn title_on(date: Date) -> Headless<Parlor> {
    rules::fix_seed(Some(SEED));
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(date);
    menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h.app.ctx.push(haunted_hallways::title());
    h.frame(0.0);
    h.clear();
    h
}

fn spoke(h: &Headless<Parlor>, part: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(part))
}

#[test]
fn outside_halloween_both_rows_are_locked_and_say_when_they_open() {
    let mut h = title_on(Date::new(2026, 6, 1));
    let r = rows(&h);
    assert!(r.contains(&"The house stirs, locked".to_string()), "{r:?}");
    assert!(
        r.contains(&"The Lantern Night, locked".to_string()),
        "{r:?}"
    );
    assert!((INFO.event.unwrap())(Date::new(2026, 6, 1)).is_none());
    goto(&mut h, "The house stirs, locked");
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(&h, "The house stirs from Saturday 17 October, in 138 days."),
        "{:?}",
        h.spoken()
    );
    goto(&mut h, "The Lantern Night, locked");
    h.clear();
    h.press(Key::Return);
    assert!(
        spoke(
            &h,
            "The Lantern Night opens Friday 30 October, in 151 days."
        ),
        "{:?}",
        h.spoken()
    );
    assert_eq!(h.app.top_name(), "haunted_hallways");
}

#[test]
fn the_halloween_rows_keep_their_places_on_every_date() {
    let at = |date| {
        let r = rows(&title_on(date));
        let i = r.iter().position(|x| x.starts_with(STIRS)).unwrap();
        let j = r.iter().position(|x| x.starts_with(NIGHT)).unwrap();
        (r.len(), i, j)
    };
    let june = at(Date::new(2026, 6, 1));
    assert_eq!(june, at(Date::new(2026, 10, 22)));
    assert_eq!(june, at(Date::new(2026, 10, 31)));
    assert_eq!(june, at(Date::new(2026, 11, 15)));
}

#[test]
fn every_candle_lit_keeps_the_run_up_row() {
    assert_eq!(stirs_done_label(), "The house stirs, all 13 candles lit");
    assert_eq!(
        stirs_done_line(d(10, 29)),
        "All 13 candles are lit. The Lantern Night opens tomorrow."
    );
}

#[test]
fn in_the_run_up_the_lantern_night_is_locked_and_counts_down() {
    let mut h = title_on(Date::new(2026, 10, 22));
    goto(&mut h, "The Lantern Night, locked");
    h.press(Key::Return);
    assert!(
        spoke(
            &h,
            "The Lantern Night opens in 8 days, the night before Halloween."
        ),
        "{:?}",
        h.spoken()
    );
    assert_eq!(h.app.top_name(), "haunted_hallways");
}

#[test]
fn the_house_stirs_in_the_run_up_and_a_floor_opens_with_lantern_jack() {
    let date = Date::new(2026, 10, 22);
    let mut h = title_on(date);
    let floor = allhallows::candle_floor(2026, 1);
    let label = format!(
        "{}, candle 1 of {CANDLES}: floor {floor}",
        allhallows::STIRS
    );
    assert!(rows(&h).contains(&label), "{:?}", rows(&h));
    assert_eq!(
        (INFO.event.unwrap())(date).unwrap().title,
        allhallows::STIRS
    );
    goto(&mut h, allhallows::STIRS);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "haunt");
    assert!(
        spoke(&h, "The house stirs: Lantern Jack"),
        "{:?}",
        h.spoken()
    );
    // The opening names the restless ghosts; no twist line repeats it.
    assert!(!spoke(&h, "Restless dead"), "{:?}", h.spoken());
}

#[test]
fn a_lit_candle_moves_the_row_to_the_next_one() {
    let date = Date::new(2026, 10, 22);
    let mut h = title_on(date);
    let mut ledger = Ledger::load(&mut h.app.ctx);
    ledger.light(2026);
    ledger.store(&mut h.app.ctx);
    h.app.ctx.pop();
    h.app.ctx.push(haunted_hallways::title());
    h.frame(0.0);
    assert!(
        rows(&h).iter().any(|r| r.contains("candle 2 of 13")),
        "{:?}",
        rows(&h)
    );
}

#[test]
fn the_lantern_night_opens_on_halloween_with_the_candles_lit() {
    let date = Date::new(2026, 10, 31);
    let mut h = title_on(date);
    let mut ledger = Ledger::load(&mut h.app.ctx);
    for _ in 0..3 {
        ledger.light(2026);
    }
    ledger.store(&mut h.app.ctx);
    h.app.ctx.pop();
    h.app.ctx.push(haunted_hallways::title());
    h.frame(0.0);
    h.clear();
    assert!(
        rows(&h).iter().any(|r| r == allhallows::NIGHT),
        "{:?}",
        rows(&h)
    );
    goto(&mut h, allhallows::NIGHT);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "haunt");
    assert!(spoke(&h, "The Lantern Night."), "{:?}", h.spoken());
    assert!(spoke(&h, "3 candles"), "{:?}", h.spoken());
}

#[test]
fn the_lantern_night_is_thirteen_dark_floors_with_more_time_per_candle() {
    let none = Mode::LanternNight(0);
    let some = Mode::LanternNight(4);
    assert_eq!(none.first_floor(), 1);
    assert_eq!(none.last_floor(), CANDLES);
    assert!(none.carries_courage());
    let (a, b) = (none.spec(5), some.spec(5));
    assert!(a.lights_out);
    assert_eq!(b.haunt - a.haunt, 4.0 * allhallows::CANDLE_SECS);
    assert_eq!(none.board(), allhallows::NIGHT_BOARD);
    assert_eq!(Mode::Stirs(7).board(), allhallows::STIRS_BOARD);
    let _ = Haunt::new(Mode::Stirs(7));
}

fn d(m: u32, day: u32) -> Date {
    Date::new(2026, m, day)
}

#[test]
fn the_house_stirs_from_17_october_and_the_night_opens_on_the_30th() {
    assert!(!stirs(d(10, 16)));
    assert!(stirs(d(10, 17)));
    assert!(!is_night(d(10, 29)));
    assert!(is_night(d(10, 30)));
    assert!(is_night(d(11, 1)));
    assert!(!is_night(d(11, 2)));
    assert!(!stirs(d(11, 2)));
}

#[test]
fn the_launcher_names_each_part() {
    assert!(event(d(10, 16)).is_none());
    let e = event(d(10, 22)).unwrap();
    assert_eq!(e.title, STIRS);
    assert!(e.line.contains("in 8 days"), "{}", e.line);
    assert!(event(d(10, 29)).unwrap().line.contains("tomorrow"));
    assert_eq!(event(d(10, 31)).unwrap().title, NIGHT);
    assert!(event(d(11, 2)).is_none());
}

#[test]
fn candle_counts_read_as_words() {
    assert_eq!(
        candles_words(1),
        "Your 1 candle adds 10 seconds to every haunt clock."
    );
    assert!(candles_words(3).starts_with("Your 3 candles add"));
    assert!(night_opening(1).contains("Your 1 candle adds"));
    assert!(night_help(d(10, 31), 1).contains("Your 1 candle adds"));
    assert!(stirs_opening().ends_with("Clear this floor to light a candle."));
}

#[test]
fn the_stairs_say_what_leaving_keeps_in_each_mode() {
    use haunted_hallways::haunt::leave_words;
    assert!(leave_words(Mode::Climb(1)).contains("this floor saved"));
    assert!(leave_words(Mode::LanternNight(3)).contains("starts at floor 1"));
    assert!(!leave_words(Mode::LanternNight(3)).contains("saved"));
    assert!(leave_words(Mode::Nightmare).contains("deepest floor saved"));
}

#[test]
fn candle_floors_are_fixed_and_deepen() {
    for c in 1..=CANDLES {
        let f = candle_floor(2026, c);
        assert_eq!(f, candle_floor(2026, c));
        assert!((1 + (c - 1) * 2..=3 + (c - 1) * 2).contains(&f));
    }
}

#[test]
fn the_run_up_row_names_the_next_candle_until_all_are_lit() {
    let (label, help, floor) = stirs_row(d(10, 20), 4).unwrap();
    assert!(label.contains("candle 5 of 13"), "{label}");
    assert!(label.ends_with(&format!("floor {floor}")));
    assert!(help.contains("opens in 10 days"), "{help}");
    assert!(stirs_row(d(10, 31), 12).is_some());
    assert!(stirs_row(d(10, 31), CANDLES).is_none());
    assert!(stirs_row(d(10, 10), 0).is_none());
}

#[test]
fn the_ledger_lights_candles_and_keeps_the_ember() {
    let mut l = Ledger::default();
    for _ in 0..20 {
        l.light(2026);
    }
    assert_eq!(l.year(2026).candles, CANDLES);
    assert!(!l.night_done(2026, false));
    assert!(l.night_done(2026, true));
    assert!(!l.night_done(2026, true));
    assert!(l
        .page()
        .contains("2026, 13 of 13 candles lit, Lantern Jack's ember kept"));
    assert_eq!(l.year(2027).candles, 0);
}

#[test]
fn the_locked_row_counts_down() {
    assert_eq!(
        locked_line(d(10, 29)),
        "The Lantern Night opens tomorrow, the night before Halloween. Light candles for it while the house stirs."
    );
    assert_eq!(
        locked_line(d(10, 20)),
        "The Lantern Night opens in 10 days, the night before Halloween. Light candles for it while the house stirs."
    );
    assert_eq!(
        locked_line(d(7, 15)),
        "The Lantern Night opens Friday 30 October, in 107 days."
    );
}
