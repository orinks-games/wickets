use hearken::{Headless, Key, LocalTime};
use parlor_core::menus::{labels, rows_of};
use parlor_core::{Date, Parlor, ParlorData, Records};
use wickets::{Lobby, TodayList, LAST_GAME_KEY};

fn rows(h: &Headless<Parlor>) -> Vec<String> {
    rows_of(&h.app.top_lines())
}

/// The lobby on `today`, at 9 PM.
fn start_on(today: Date) -> Headless<Parlor> {
    let now = format!("{}-{:02}-{:02}T21:00", today.year, today.month, today.day);
    let data = ParlorData {
        today: Some(today),
        now: LocalTime::parse(&now),
        ..Default::default()
    };
    Headless::new(data, Lobby::new())
}

fn select(h: &mut Headless<Parlor>, label: &str) {
    let at = rows(h)
        .iter()
        .position(|r| r.starts_with(label))
        .unwrap_or_else(|| panic!("no {label} in {:?}", rows(h)));
    h.press(Key::Home);
    for _ in 0..at {
        h.press(Key::Down);
    }
    h.press(Key::Return);
}

const VALENTINES: Date = Date::new(2027, 2, 14);
const QUIET_DAY: Date = Date::new(2027, 6, 10);

#[test]
fn the_lobby_row_counts_todays_events() {
    let h = start_on(VALENTINES);
    assert_eq!(rows(&h)[1], "Happening today, 2 events");
    let mut h = start_on(Date::new(2026, 10, 26));
    let all = wickets::happenings(&mut h.app.ctx).len();
    assert_eq!(rows(&h)[1], format!("Happening today, {all} events"));
    let h = start_on(QUIET_DAY);
    assert_eq!(rows(&h)[1], "Happening today");
}

#[test]
fn the_list_holds_events_then_the_challenges() {
    let mut h = start_on(VALENTINES);
    select(&mut h, "Happening today");
    assert_eq!(h.app.top_name(), "today");
    let rows = rows(&h);
    assert_eq!(rows[0], "Cupid's Quiver, Valentine's night", "{rows:?}");
    assert!(rows[1].starts_with("Slow Kettle, "), "{rows:?}");
    assert!(rows[2].starts_with("Daily challenges, 0 of "), "{rows:?}");
    assert!(rows[3].starts_with("Weekly challenges, "), "{rows:?}");
    assert!(rows[4].starts_with("Monthly challenges, "), "{rows:?}");
    assert_eq!(rows[5], labels::BACK);
    assert_eq!(rows.len(), 6);
}

#[test]
fn every_busy_day_event_is_listed() {
    let mut h = start_on(Date::new(2026, 10, 26));
    let all = wickets::happenings(&mut h.app.ctx);
    assert!(all.len() > 3);
    h.app.ctx.push(TodayList::new());
    h.frame(0.0);
    let rows = rows(&h);
    for (g, e) in all {
        let label = format!("{}, {}", g.name, wickets::after_comma(&e.title));
        assert!(rows.contains(&label), "{label} not in {rows:?}");
    }
}

#[test]
fn enter_on_an_event_opens_its_game() {
    let mut h = start_on(VALENTINES);
    select(&mut h, "Happening today");
    h.press(Key::Home);
    h.press(Key::Return);
    assert_eq!(h.app.depth(), 3);
    let last: Option<String> = Records::load(&mut h.app.ctx, LAST_GAME_KEY);
    assert_eq!(last.as_deref(), Some("cupids-quiver"));
    // Leaving the game comes back to the list.
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "today");
}

#[test]
fn a_quiet_day_names_the_next_event() {
    let mut h = start_on(QUIET_DAY);
    select(&mut h, "Happening today");
    assert_eq!(rows(&h)[0], "No events today");
    h.clear();
    h.press(Key::Home);
    h.press(Key::Return);
    let said = h.spoken().join(" ");
    assert!(said.contains("No events today. Next: "), "{said}");
    let (g, title, days) =
        wickets::today::next_event(&mut h.app.ctx, QUIET_DAY).expect("an event soon");
    let named = format!("{}, {}", g.name, wickets::after_comma(&title));
    assert!(said.contains(&named), "{said}");
    assert!(days > 0);
}

#[test]
fn a_challenge_row_opens_that_periods_list() {
    let mut h = start_on(QUIET_DAY);
    select(&mut h, "Happening today");
    select(&mut h, "Weekly challenges");
    assert_eq!(h.app.top_name(), "challenge list");
    assert_eq!(h.app.top_lines()[0], "Weekly challenges");
}

#[test]
fn an_event_opens_its_title_on_the_event_row() {
    let mut h = start_on(Date::new(2026, 10, 24));
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, "haunted-hallways");
    select(&mut h, "Happening today");
    select(&mut h, "Nevermore Manor, the house stirs");
    assert_eq!(h.app.top_name(), "haunted_hallways");
    let lines = h.app.top_lines();
    let current = lines
        .iter()
        .find(|l| l.starts_with("> "))
        .expect("a current row");
    assert!(current.starts_with("> The house stirs"), "{lines:?}");
}

/// Every game event on every day of a year, opened the way Happening
/// today and story mode open it, lands on a row the player can play: the
/// event's own row, never a locked one, or Play when Play holds the event.
/// A row that plays the event under another name must say so with
/// `Happening::on_row`, or the title would open on Play past it.
///
/// Events Play holds although another row shares a word with them.
const PLAY_HOLDS: [(&str, &str); 1] = [
    // The seed packet in the garden holds the pumpkin, not the Seed shed.
    ("rooftop-garden", "Pumpkin seeds"),
];

#[test]
fn every_event_opens_its_title_on_a_row_that_plays_it() {
    let start = Date::new(2026, 1, 1).days();
    let mut wrong = Vec::new();
    for d in 0..366 {
        let date = Date::from_days(start + d);
        let mut all = wickets::games();
        all.extend(wickets::specials().into_iter().map(|(g, _)| g));
        for g in all {
            let Some(event) = g.event.and_then(|e| e(date)) else {
                continue;
            };
            let mut h = start_on(date);
            parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, g.id);
            parlor_core::tutorial::mark_played(&mut h.app.ctx, g.id);
            let focus = event.focus().to_lowercase();
            h.app.ctx.game.title_focus = Some(event.focus());
            (g.open)(&mut h.app.ctx);
            h.frame(0.0);
            let lines = h.app.top_lines();
            let current = lines
                .iter()
                .find_map(|l| l.strip_prefix("> "))
                .unwrap_or_default()
                .to_lowercase();
            let all_rows = rows(&h);
            let named = all_rows
                .iter()
                .any(|r| r.to_lowercase().starts_with(&focus));
            // A row that shares the event's last word may play it under
            // another name.
            let last_word = focus.rsplit(' ').next().unwrap_or_default().to_string();
            let near_miss = !named
                && !PLAY_HOLDS.contains(&(g.id, event.title.as_str()))
                && all_rows.iter().skip(1).any(|r| {
                    r.to_lowercase()
                        .split(|c: char| !c.is_alphanumeric() && c != '\'')
                        .any(|w| w == last_word)
                });
            let ok = if named {
                current.starts_with(&focus) && !current.contains("locked")
            } else {
                current.starts_with("play") && !near_miss
            };
            if !ok {
                wrong.push(format!(
                    "{date:?} {}: {} on {current:?} of {all_rows:?}",
                    g.id, event.title
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn a_kind_left_out_in_preferences_leaves_events_but_not_play() {
    use parlor_core::{preferences, Category};
    let mut h = start_on(VALENTINES);
    assert!(preferences::set(&mut h.app.ctx, Category::Seasonal, false));
    h.app.ctx.push(TodayList::new());
    h.frame(0.0);
    let today = rows(&h);
    assert!(today[0].starts_with("Slow Kettle, "), "{today:?}");
    assert!(
        !today.iter().any(|r| r.starts_with("Cupid's Quiver")),
        "{today:?}"
    );
    // Play still lists the game and its event.
    let mut h = start_on(VALENTINES);
    assert!(preferences::set(&mut h.app.ctx, Category::Seasonal, false));
    h.app.ctx.push(wickets::GameList::new(Category::Seasonal));
    h.frame(0.0);
    let games = rows(&h);
    assert!(
        games
            .iter()
            .any(|r| r == "Cupid's Quiver, Valentine's night today"),
        "{games:?}"
    );
}

#[test]
fn preferences_open_from_the_launchers_settings() {
    let mut h = start_on(QUIET_DAY);
    select(&mut h, "Settings");
    let settings = rows(&h);
    let row = settings
        .iter()
        .find(|r| r.starts_with("Story and event games"))
        .unwrap_or_else(|| panic!("{settings:?}"));
    assert_eq!(row, "Story and event games: all on");
    select(&mut h, "Story and event games");
    assert_eq!(h.app.top_name(), "preferences");
    select(&mut h, "Table games");
    assert_eq!(h.app.top_name(), "preferences_category");
    select(&mut h, "Fireside Patience");
    assert_eq!(
        h.spoken().last().unwrap(),
        "Fireside Patience: off. Still in Play."
    );
    h.press(Key::Escape);
    assert!(
        rows(&h).contains(&"Table games: 1 of 2 on".to_string()),
        "{:?}",
        rows(&h)
    );
    h.press(Key::Escape);
    let settings = rows(&h);
    assert!(
        settings
            .iter()
            .any(|r| r == "Story and event games: 1 game off"),
        "{settings:?}"
    );
}

#[test]
fn a_single_game_left_out_leaves_happening_today() {
    use parlor_core::preferences;
    let mut h = start_on(VALENTINES);
    let kettle = wickets::games()
        .into_iter()
        .find(|g| g.id == "slow-kettle")
        .unwrap();
    assert!(preferences::set_game(&mut h.app.ctx, &kettle, false));
    h.app.ctx.push(TodayList::new());
    h.frame(0.0);
    let today = rows(&h);
    assert_eq!(today[0], "Cupid's Quiver, Valentine's night", "{today:?}");
    assert!(today[1].starts_with("Daily challenges, "), "{today:?}");
}
