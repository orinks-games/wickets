//! Catch-up changes base-game visibility, not holiday activities or progression.
use hearken::{Headless, Key};
use parlor_core::menus::rows_of;
use parlor_core::{Category, Date, Parlor, ParlorData};
use wickets::discovery_clock::{simulate, status, Status};
use wickets::{Access, CategoryList, GameList, Keeper, Lobby};

fn on(date: Date) -> Headless<Parlor> {
    Headless::new(
        ParlorData {
            today: Some(date),
            ..Default::default()
        },
        Lobby::new(),
    )
}
fn keeper(id: &str) -> Keeper {
    wickets::keepers()
        .into_iter()
        .find(|k| k.game.id == id)
        .unwrap()
}
fn shown(h: &mut Headless<Parlor>, id: &str) -> bool {
    wickets::specials_listed(&mut h.app.ctx)
        .iter()
        .any(|g| g.id == id)
}

#[test]
fn halloween_catch_up_covers_release_week_but_has_a_firm_end() {
    let k = keeper(graveyard_shuffle::ID);
    for date in [
        Date::new(2026, 10, 17),
        Date::new(2026, 11, 1),
        Date::new(2026, 11, 7),
        Date::new(2026, 11, 15),
    ] {
        assert!(k.available(date));
        assert!(shown(&mut on(date), graveyard_shuffle::ID));
    }
    for date in [
        Date::new(2026, 10, 16),
        Date::new(2026, 11, 16),
        Date::new(2027, 6, 1),
    ] {
        assert!(!k.available(date));
        assert!(!shown(&mut on(date), graveyard_shuffle::ID));
    }
    let date = Date::new(2026, 11, 7);
    assert!(!(k.in_season)(date));
    assert!(graveyard_shuffle::event(date).is_none());
    assert!(!graveyard_shuffle::progress::dated(4, date));
    assert!(graveyard_shuffle::progress::dated(1, date));
}

#[test]
fn winter_catch_up_handles_february_and_leap_years() {
    let k = keeper(snowfall_skate::ID);
    for year in [2027, 2028] {
        assert!(k.available(Date::new(year, 3, 14)));
        assert!(!k.available(Date::new(year, 3, 15)));
        assert!(!(k.in_season)(Date::new(year, 3, 1)));
        assert!(!k.available(Date::new(year, 11, 30)));
    }
}

#[test]
fn every_keeper_gets_fourteen_days_after_its_actual_season_ends() {
    for k in wickets::keepers() {
        let mut date = Date::new(2026, 1, 1);
        while date < Date::new(2029, 1, 1) {
            let next = Date::from_days(date.days() + 1);
            if (k.in_season)(date) && !(k.in_season)(next) {
                for offset in 1..=14 {
                    assert!(
                        k.available(Date::from_days(date.days() + offset)),
                        "{} day {offset}",
                        k.game.id
                    );
                }
                let after = Date::from_days(date.days() + 15);
                if !(k.in_season)(after) {
                    assert!(!k.available(after), "{} beyond grace", k.game.id);
                }
            }
            date = next;
        }
    }
}

#[test]
fn old_play_progress_keeps_the_base_without_opening_the_finale() {
    let mut h = on(Date::new(2026, 11, 16));
    let p = graveyard_shuffle::Progress {
        practice_steps: 1,
        ..Default::default()
    };
    p.store(&mut h.app.ctx);
    let listed = wickets::specials_listed(&mut h.app.ctx);
    let g = listed
        .iter()
        .find(|g| g.id == graveyard_shuffle::ID)
        .unwrap();
    assert_eq!(g.category, Category::Action);
    assert!(
        !p.set_open(2),
        "retention must not grant stars or progression"
    );
    assert!(!p.night_open(4, Date::new(2026, 11, 16)));
}

#[test]
fn listing_does_not_count_as_play_or_permanently_reveal_a_title() {
    let mut h = on(Date::new(2026, 11, 7));
    assert!(shown(&mut h, graveyard_shuffle::ID));
    assert!(!graveyard_shuffle::kept(&mut h.app.ctx));
    h.app.ctx.game.today = Some(Date::new(2026, 11, 16));
    assert!(!shown(&mut h, graveyard_shuffle::ID));
}

/// The launcher on the real date, with the internet date `network`
/// simulated on this test's thread.
fn real_date(network: Status) -> Headless<Parlor> {
    simulate(Some(network));
    Headless::new(ParlorData::default(), Lobby::new())
}

fn seasonal_list(h: &mut Headless<Parlor>) {
    h.app.ctx.push(CategoryList::new());
    h.app.ctx.push(GameList::new(Category::Seasonal));
    h.frame(0.0);
    h.clear();
}

/// Seasonal games offline: opening the list starts a check, and here it
/// fails.
fn offline_seasonal_list(h: &mut Headless<Parlor>) {
    seasonal_list(h);
    assert_eq!(status(), Status::Checking, "opening the list checks");
    assert!(rows(h).iter().any(|r| r == "Checking for holiday specials"));
    simulate(Some(Status::Unavailable));
    h.frame(0.1);
    assert!(h.spoken().is_empty(), "a check nobody asked for is quiet");
    h.clear();
}

fn rows(h: &Headless<Parlor>) -> Vec<String> {
    rows_of(&h.app.top_lines())
}

fn goto(h: &mut Headless<Parlor>, label: &str) {
    let at = rows(h)
        .iter()
        .position(|r| r.starts_with(label))
        .unwrap_or_else(|| panic!("no row {label:?}: {:?}", rows(h)));
    h.press(Key::Home);
    for _ in 0..at {
        h.press(Key::Down);
    }
}

fn said(h: &Headless<Parlor>, text: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(text))
}

#[test]
fn offline_hides_new_specials_but_keeps_played_ones() {
    let mut h = real_date(Status::Unavailable);
    // Whatever the computer's clock says, no internet date finds nothing new.
    assert!(!shown(&mut h, graveyard_shuffle::ID));
    assert_eq!(
        wickets::access(&mut h.app.ctx, graveyard_shuffle::ID),
        Access::NeedsDate
    );
    graveyard_shuffle::Progress::note_played(&mut h.app.ctx);
    assert!(shown(&mut h, graveyard_shuffle::ID));
    assert_eq!(
        wickets::access(&mut h.app.ctx, graveyard_shuffle::ID),
        Access::Open
    );
    simulate(None);
}

#[test]
fn checking_counts_as_no_date() {
    let mut h = real_date(Status::Checking);
    assert!(!shown(&mut h, graveyard_shuffle::ID));
    simulate(None);
}

#[test]
fn the_internet_date_decides_not_the_computer_clock() {
    // Grace on the internet date opens it; a day past shuts it.
    let mut h = real_date(Status::Ready(Date::new(2026, 11, 15)));
    assert!(shown(&mut h, graveyard_shuffle::ID));
    simulate(Some(Status::Ready(Date::new(2026, 11, 16))));
    assert!(!shown(&mut h, graveyard_shuffle::ID));
    assert_eq!(
        wickets::access(&mut h.app.ctx, graveyard_shuffle::ID),
        Access::Closed
    );
    simulate(None);
}

#[test]
fn ordinary_games_never_need_the_date() {
    let mut h = real_date(Status::Unavailable);
    for g in wickets::games() {
        assert_eq!(wickets::access(&mut h.app.ctx, g.id), Access::Open, "{}", g.id);
    }
    simulate(None);
}

#[test]
fn offline_seasonal_list_offers_a_check_and_answers_it() {
    let mut h = real_date(Status::Unavailable);
    offline_seasonal_list(&mut h);
    assert!(rows(&h).iter().any(|r| r == "Check for holiday specials"));
    assert!(!rows(&h).iter().any(|r| r.starts_with("Graveyard Shuffle")));
    goto(&mut h, "Check for holiday specials");
    h.clear();
    h.press(Key::Return);
    assert!(said(&h, "Checking."), "{:?}", h.spoken());
    assert_eq!(status(), Status::Checking);
    h.frame(0.1);
    assert!(rows(&h).iter().any(|r| r == "Checking for holiday specials"));
    // The answer arrives while the list is open.
    h.clear();
    simulate(Some(Status::Ready(Date::new(2026, 11, 10))));
    h.frame(0.1);
    assert!(said(&h, "Holiday specials checked."), "{:?}", h.spoken());
    assert!(rows(&h).iter().any(|r| r.starts_with("Graveyard Shuffle")));
    assert!(!rows(&h).iter().any(|r| r.contains("holiday specials")));
    simulate(None);
}

#[test]
fn a_failed_check_says_so_and_keeps_the_row() {
    let mut h = real_date(Status::Unavailable);
    offline_seasonal_list(&mut h);
    goto(&mut h, "Check for holiday specials");
    h.press(Key::Return);
    h.frame(0.1);
    h.clear();
    simulate(Some(Status::Unavailable));
    h.frame(0.1);
    assert!(said(&h, "No internet date. Try again later."), "{:?}", h.spoken());
    assert!(rows(&h).iter().any(|r| r == "Check for holiday specials"));
    simulate(None);
}

#[test]
fn a_stale_list_cannot_open_a_special_after_its_window() {
    let mut h = real_date(Status::Ready(Date::new(2026, 11, 15)));
    seasonal_list(&mut h);
    goto(&mut h, "Graveyard Shuffle");
    // Midnight passes on the internet date while the list sits open, so
    // the row is stale when the player presses Enter.
    simulate(Some(Status::Ready(Date::new(2026, 11, 16))));
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "games", "the game must not open");
    assert!(said(&h, "Graveyard Shuffle is back next season."), "{:?}", h.spoken());
    assert!(!graveyard_shuffle::kept(&mut h.app.ctx));
    h.frame(0.1);
    assert!(!rows(&h).iter().any(|r| r.starts_with("Graveyard Shuffle")));
    simulate(None);
}

#[test]
fn a_stale_list_needs_the_date_when_it_is_lost() {
    let mut h = real_date(Status::Ready(Date::new(2026, 11, 10)));
    seasonal_list(&mut h);
    goto(&mut h, "Graveyard Shuffle");
    simulate(Some(Status::Unavailable));
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "games");
    assert!(said(&h, wickets::NEEDS_DATE), "{:?}", h.spoken());
    simulate(None);
}

#[test]
fn a_stale_continue_row_is_checked_too() {
    let mut h = real_date(Status::Ready(Date::new(2026, 11, 10)));
    parlor_core::Records::store(
        &mut h.app.ctx,
        wickets::LAST_GAME_KEY,
        &Some(graveyard_shuffle::ID),
    );
    // Back in the lobby, its rows are built again.
    h.app.ctx.push(CategoryList::new());
    h.frame(0.0);
    h.press(Key::Escape);
    h.frame(0.0);
    assert!(rows(&h)[0].starts_with("Continue, Graveyard Shuffle"), "{:?}", rows(&h));
    simulate(Some(Status::Unavailable));
    h.press(Key::Home);
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "lobby");
    assert!(said(&h, wickets::NEEDS_DATE), "{:?}", h.spoken());
    simulate(None);
}

#[test]
fn a_kept_special_opens_offline_from_a_list() {
    let mut h = real_date(Status::Unavailable);
    graveyard_shuffle::Progress::note_played(&mut h.app.ctx);
    h.app.ctx.push(CategoryList::new());
    h.app.ctx.push(GameList::new(graveyard_shuffle::HOME));
    h.frame(0.0);
    let listed = rows(&h).iter().any(|r| r.starts_with("Graveyard Shuffle"));
    if !listed {
        // In its season by the computer's clock, it is in Seasonal games.
        h.app.ctx.pop();
        h.app.ctx.push(GameList::new(Category::Seasonal));
        h.frame(0.0);
    }
    goto(&mut h, "Graveyard Shuffle");
    h.press(Key::Return);
    assert_ne!(h.app.top_name(), "games", "a kept game opens offline");
    simulate(None);
}

#[test]
fn winter_catch_up_crosses_the_new_year() {
    let k = keeper(snowfall_skate::ID);
    assert!(k.available(Date::new(2026, 12, 31)));
    assert!(k.available(Date::new(2027, 1, 1)));
    assert_eq!(
        k.available(Date::new(2027, 3, 14)),
        !(k.in_season)(Date::new(2027, 3, 15))
    );
}
