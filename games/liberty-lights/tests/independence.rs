//! Independence night on the title: locked with a countdown most of the
//! year, a rehearsal in the week before, a four-act show on the Fourth,
//! and a keepsake for each year played.

use hearken::{Ctx, Headless, InputEvent, Key, State};
use liberty_lights::independence::{self, Keepsakes, NightRun};
use liberty_lights::rules::{self, Kind, Launch, Pos};
use liberty_lights::{title, Mode, Show, ID, INFO};
use parlor_core::{menus, testkit, Date, Parlor, Records};

struct Base;

impl State<Parlor> for Base {
    fn name(&self) -> &'static str {
        "base"
    }
    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        String::new()
    }
}

/// The title on `date`, tutorial read, nothing spoken yet.
fn title_on(date: Date) -> Headless<Parlor> {
    rules::fix_seed(Some(4));
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(date);
    menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h.app.ctx.push(title());
    h.frame(0.0);
    h.clear();
    h
}

fn rows(h: &Headless<Parlor>) -> Vec<String> {
    menus::rows_of(&h.app.top_lines())
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

fn spoke(h: &Headless<Parlor>, part: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(part))
}

fn digit(kind: Kind) -> Key {
    match kind {
        Kind::Boom => Key::Num1,
        Kind::Crackle => Key::Num2,
        Kind::Whistle => Key::Num3,
        Kind::Sparkle => Key::Num4,
    }
}

/// Hears the show on top through to the turn, then launches `launches`
/// on time and finishes.
fn play_act(h: &mut Headless<Parlor>, launches: &[Launch]) {
    h.press(Key::Space);
    for _ in 0..2000 {
        if spoke(h, "Your turn.") {
            break;
        }
        h.frame(0.05);
    }
    assert!(spoke(h, "Your turn."), "{:?}", h.spoken());
    h.press(Key::Space);
    let mut ms = -1500;
    for l in launches {
        while ms < l.ms {
            h.frame(0.01);
            ms += 10;
        }
        let hold = match l.pos {
            Pos::Left => Some(Key::Left),
            Pos::Right => Some(Key::Right),
            Pos::Center => None,
        };
        if let Some(k) = hold {
            h.send(InputEvent::key(k));
        }
        h.press(digit(l.kind));
        if let Some(k) = hold {
            h.send(InputEvent::key_up(k));
        }
    }
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "results");
}

#[test]
fn the_launcher_names_the_night_only_on_the_day() {
    let event = INFO.event.expect("Liberty Lights holds an event");
    for year in [2026, 2027, 2030] {
        let on = event(Date::new(year, 7, 4)).expect("on the Fourth");
        assert_eq!(on.title, "Independence night");
        assert!(on.line.ends_with('.'));
        assert!(event(Date::new(year, 7, 3)).is_some());
        assert!(event(Date::new(year, 7, 5)).is_some());
        assert!(event(Date::new(year, 7, 2)).is_none());
        assert!(event(Date::new(year, 7, 6)).is_none());
        assert!(event(Date::new(year, 1, 1)).is_none());
    }
}

#[test]
fn the_row_is_locked_with_a_countdown_the_rest_of_the_year() {
    let mut h = title_on(Date::new(2026, 10, 16));
    goto(&mut h, "Independence night");
    assert_eq!(rows(&h)[4], "Independence night, locked");
    h.clear();
    h.press(Key::F1);
    assert!(
        spoke(&h, "The Fourth of July is in 261 days."),
        "{:?}",
        h.spoken()
    );
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights");
    assert_eq!(
        h.spoken(),
        ["Independence night opens on the Fourth of July. The Fourth of July is in 261 days."]
    );
    assert!(
        !h.played().is_empty(),
        "a locked row sounds as in the other games"
    );
}

#[test]
fn the_week_before_plays_a_rehearsal_of_the_salute() {
    let mut h = title_on(Date::new(2027, 6, 30));
    goto(&mut h, "Independence night");
    assert_eq!(rows(&h)[4], "Independence night, rehearsal");
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "The Fourth of July is in 4 days."));
    assert!(spoke(&h, "rehearsal"));
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_replay");
    assert!(spoke(&h, "Rehearsal: the opening salute"));
    for _ in 0..400 {
        h.frame(0.05);
    }
    assert_eq!(h.app.top_name(), "liberty_lights");
    assert!(h
        .played()
        .iter()
        .any(|k| k == "liberty-lights/fireworks/boom"));
}

#[test]
fn rows_keep_their_places_before_on_and_after_the_day() {
    let dates = [
        Date::new(2026, 7, 2),
        Date::new(2026, 7, 3),
        Date::new(2026, 7, 4),
        Date::new(2026, 7, 5),
        Date::new(2026, 7, 6),
        Date::new(2027, 1, 1),
    ];
    let lists: Vec<Vec<String>> = dates.iter().map(|&d| rows(&title_on(d))).collect();
    for (d, list) in dates.iter().zip(&lists) {
        assert_eq!(list.len(), lists[0].len(), "{d:?}");
        assert!(list[4].starts_with("Independence night"), "{d:?} {list:?}");
        let open = independence::is_open(*d);
        let shut = list[4].ends_with(", locked") || list[4].ends_with(", rehearsal");
        assert_eq!(!shut, open, "{d:?}");
    }
    assert!(!independence::is_open(Date::new(2026, 7, 2)));
    assert!(independence::is_open(Date::new(2026, 7, 5)));
}

#[test]
fn on_the_fourth_the_night_opens_with_the_salute() {
    let mut h = title_on(Date::new(2026, 7, 4));
    goto(&mut h, "Independence night");
    h.clear();
    h.press(Key::F1);
    assert!(spoke(&h, "Four acts for the Fourth of July"));
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_show");
    assert!(
        spoke(
            &h,
            "Independence night, act 1 of 4: The opening salute. 6 fireworks."
        ),
        "{:?}",
        h.spoken()
    );
    assert!(h.played().iter().any(|k| k == "liberty-lights/cue/salute"));
    h.clear();
    h.press(Key::S);
    h.press(Key::S);
    assert!(spoke(&h, "Signature firework: boom."));
    testkit::every_action_answers(&mut h);
}

#[test]
fn a_whole_night_is_kept_as_this_years_keepsake() {
    let mut h = title_on(Date::new(2026, 7, 5));
    goto(&mut h, "Independence night");
    h.press(Key::Return);
    let acts = independence::acts(2026);
    for (i, act) in acts.iter().enumerate() {
        assert_eq!(h.app.top_name(), "liberty_lights_show");
        play_act(&mut h, &act.show);
        let said = h.spoken()[0].clone();
        assert!(
            said.starts_with(&format!("{} cleared, 100 percent, ", act.name)),
            "{said}"
        );
        if i == 0 {
            goto(&mut h, "Hear the results again");
            h.clear();
            h.press(Key::Return);
            h.press(Key::Return);
            assert!(
                spoke(&h, "The booms paid 600 bonus points."),
                "{:?}",
                h.spoken()
            );
            h.press(Key::Return);
            h.press(Key::Return);
        }
        if i + 1 < acts.len() {
            assert!(said.contains("Night total "), "{said}");
            h.press(Key::Home);
            h.press(Key::Return);
        } else {
            assert!(said.contains("The night is over: "), "{said}");
            assert!(said.contains("Your keepsake for 2026 is saved."), "{said}");
        }
    }
    let keep = Keepsakes::load(&mut h.app.ctx);
    let k = keep.year(2026).expect("kept");
    assert_eq!((k.acts_cleared, k.nights), (4, 1));
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, independence::BOARD),
        Some(k.best)
    );
    // Back on the title, Records reads it.
    h.press(Key::End);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights");
    goto(&mut h, "Records");
    h.press(Key::Return);
    let mut pages = Vec::new();
    for _ in 0..10 {
        pages.extend(h.spoken());
        h.clear();
        if h.app.top_name() != "pages" {
            break;
        }
        h.press(Key::Return);
    }
    let all = pages.join(" ");
    assert!(
        all.contains("Keepsakes. Independence night 2026: best"),
        "{all}"
    );
}

#[test]
fn each_year_keeps_its_own_keepsake() {
    let mut h = title_on(Date::new(2027, 7, 4));
    let mut run = NightRun::new(2027);
    run.stage = 3;
    let show = run.act().show.clone();
    h.app
        .ctx
        .push(Show::with_show(Mode::Independence(Box::new(run)), show));
    h.frame(0.0);
    play_act(&mut h, &[]);
    assert!(spoke(&h, "Your keepsake for 2027 is saved."));
    let keep = Keepsakes::load(&mut h.app.ctx);
    assert!(keep.year(2026).is_none());
    assert_eq!(keep.year(2027).map(|k| k.nights), Some(1));
    let page = keep.page(Date::new(2027, 7, 4));
    assert!(page.contains("Independence night 2027"), "{page}");
}

#[test]
fn the_first_act_names_the_aim_keys() {
    let mut h = title_on(Date::new(2026, 7, 4));
    goto(&mut h, "Independence night");
    h.press(Key::Return);
    h.clear();
    h.press(Key::Space);
    for _ in 0..2000 {
        if spoke(&h, "Your turn.") {
            break;
        }
        h.frame(0.05);
    }
    assert!(spoke(&h, "to launch on the left"), "{:?}", h.spoken());
}
