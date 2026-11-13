//! The daily, weekly and monthly challenges, and the controller's way to
//! play a show, driven through the screens.

use hearken::{Call, ControllerButton, Ctx, Headless, InputEvent, Key, State};
use liberty_lights::festival::{Act, Plan, Run, Twist};
use liberty_lights::rules::{self, generate, Kind, Launch, Pos};
use liberty_lights::{Mode, Show, ID};
use parlor_core::menus;
use parlor_core::{Challenge, Challenges, Date, Parlor, ParlorData, Period};

const TODAY: Date = Date::new(2026, 10, 16);

struct Base;

impl State<Parlor> for Base {
    fn name(&self) -> &'static str {
        "base"
    }
    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.say("Base.");
    }
    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        String::new()
    }
}

fn data() -> ParlorData {
    ParlorData {
        today: Some(TODAY),
        ..Default::default()
    }
}

/// The title over the launcher's stand-in, tutorial read, nothing spoken
/// yet but the title.
fn title() -> Headless<Parlor> {
    let mut h = base();
    menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h.clear();
    h.app.ctx.push(liberty_lights::title());
    h.frame(0.0);
    h
}

fn base() -> Headless<Parlor> {
    rules::fix_seed(Some(4));
    Headless::new(data(), Base)
}

fn spoke(h: &Headless<Parlor>, part: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(part))
}

#[track_caller]
fn assert_said(h: &Headless<Parlor>, part: &str) {
    assert!(spoke(h, part), "{part:?} not in {:?}", h.spoken());
}

fn last(h: &Headless<Parlor>) -> String {
    h.spoken().last().cloned().unwrap_or_default()
}

/// Moves the cursor to the first row starting with `label`.
fn goto(h: &mut Headless<Parlor>, label: &str) {
    let rows = menus::rows_of(&h.app.top_lines());
    let at = rows
        .iter()
        .position(|r| r.starts_with(label))
        .unwrap_or_else(|| panic!("no row starts with {label:?}: {rows:?}"));
    h.press(Key::Home);
    for _ in 0..at {
        h.press(Key::Down);
    }
}

/// Reads every results page with "Hear the results again", back to the
/// results menu.
fn results_pages(h: &mut Headless<Parlor>) -> String {
    goto(h, "Hear the results again");
    h.clear();
    h.press(Key::Return);
    for _ in 0..5 {
        if h.app.top_name() != "pages" {
            break;
        }
        h.press(Key::Return);
    }
    h.spoken().join(" ")
}

fn played(h: &Headless<Parlor>, key: &str) -> bool {
    h.played().iter().any(|k| k == key)
}

fn plan(h: &Headless<Parlor>, period: Period) -> Plan {
    Plan::for_challenge(&Challenge::current(&h.app.ctx, ID, period))
}

/// From the show's Ready phase (or its turn, once heard), hear the show
/// through to "your turn".
fn listen(h: &mut Headless<Parlor>) {
    h.press(Key::Space);
    for _ in 0..2000 {
        if spoke(h, "Your turn.") {
            return;
        }
        h.frame(0.05);
    }
    panic!("the show never handed over: {:?}", h.spoken());
}

fn digit(kind: Kind) -> Key {
    match kind {
        Kind::Boom => Key::Num1,
        Kind::Crackle => Key::Num2,
        Kind::Whistle => Key::Num3,
        Kind::Sparkle => Key::Num4,
    }
}

/// From "your turn": counts in and launches `launches` on time.
fn perform(h: &mut Headless<Parlor>, launches: &[Launch]) {
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
}

/// From the show's Ready phase: hear it, play it perfectly and finish.
fn play_perfect(h: &mut Headless<Parlor>, show: &[Launch]) {
    listen(h);
    perform(h, show);
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "results");
}

/// From the show's Ready phase: hear it, launch nothing and finish.
fn play_nothing(h: &mut Headless<Parlor>) {
    listen(h);
    h.press(Key::Space);
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "results");
}

/// A press and release of a controller button.
fn tap(h: &mut Headless<Parlor>, button: ControllerButton) {
    h.send(InputEvent::button(button));
    h.send(InputEvent::button_up(button));
}

/// From the title: open the challenges list.
fn challenges(h: &mut Headless<Parlor>) {
    goto(h, "Challenges");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_challenges");
}

/// From the challenges list, on `period`'s row: read the briefing through
/// to the first show.
fn brief(h: &mut Headless<Parlor>) {
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "pages");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_show");
}

#[test]
fn the_title_has_a_challenges_row_listing_all_three() {
    let mut h = title();
    h.clear();
    goto(&mut h, "Challenges");
    assert_eq!(last(&h), "Challenges, daily not played yet. 6 of 13.");
    challenges(&mut h);
    let daily = plan(&h, Period::Daily);
    assert_said(&h, "Challenges.");
    assert_said(
        &h,
        &format!(
            "Daily, Tonight's show: {}. Not played yet. 1 of 4.",
            daily.tag()
        ),
    );
    h.clear();
    h.press(Key::F1);
    assert_said(&h, "One show of");
    assert_said(&h, "The twist,");
    assert_said(&h, "A new one tomorrow.");
    h.clear();
    h.press(Key::Down);
    assert_said(
        &h,
        "Weekly, The five-show gauntlet: five shows, one failure ends it,",
    );
    h.clear();
    h.press(Key::Down);
    assert_said(
        &h,
        "Monthly, The Bonfire Parade festival: three acts ending in a 14-firework finale, crackles pay a bonus. Not played yet.",
    );
    h.clear();
    h.press(Key::F1);
    assert_said(&h, "A new one in 16 days.");
    h.press(Key::End);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights");
}

#[test]
fn the_daily_states_its_twist_scores_and_records_its_result() {
    let mut h = title();
    challenges(&mut h);
    let daily = plan(&h, Period::Daily);
    let twist = daily.twist.unwrap();
    h.clear();
    h.press(Key::Return);
    assert_said(&h, "Tonight's show: One show of");
    assert_said(
        &h,
        &format!("The twist, {}: {}.", twist.name(), twist.describe()),
    );
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_show");
    assert!(played(&h, "liberty-lights/cue/twist"));
    assert_said(
        &h,
        &format!(
            "Daily challenge: Tonight's show. {} fireworks. Twist: {}.",
            daily.acts[0].show.len(),
            twist.name()
        ),
    );
    h.clear();
    h.press(Key::S);
    h.press(Key::S);
    assert_said(&h, &format!("Twist: {}.", twist.name()));
    h.clear();
    h.press(Key::F1);
    assert_eq!(
        h.spoken(),
        [format!(
            "Daily challenge. Twist: {}. Space plays the show. F1 again for every key.",
            twist.name()
        )],
        "the first layer is short"
    );
    h.clear();
    h.press(Key::F1);
    assert_said(&h, "The twist,");

    play_perfect(&mut h, &daily.acts[0].show);
    let summary = h.spoken()[0].clone();
    assert!(
        summary.starts_with("Show cleared, 100 percent, "),
        "{summary}"
    );
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    let best = Challenges::best(&mut h.app.ctx, &ch).expect("the daily was submitted");
    assert!(best >= 1000, "{best}");
    assert!(
        summary.ends_with(&format!(
            " {best} points. Your first result on this challenge. A 1 day streak."
        )),
        "{summary}"
    );
    assert!(played(&h, "liberty-lights/music/challenge"));
    assert_eq!(
        menus::rows_of(&h.app.top_lines()),
        [
            "Play this daily challenge again",
            "Hear the results again",
            "Compare",
            "Make a sound file of this run's song",
            "Back to the challenges"
        ]
    );
    assert!(!summary.contains("A new one tomorrow."), "{summary}");
    let pages = results_pages(&mut h);
    assert!(pages.contains("The crowd cheers."), "{pages}");
    assert!(pages.contains("A new one tomorrow."), "{pages}");
    assert!(pages.contains("Firework 1 scored 100"), "{pages}");
    h.clear();
    h.press(Key::End);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_challenges");
    assert_said(&h, &format!("Done, best {best}, a 1 day streak."));
}

#[test]
fn a_worse_daily_keeps_the_best_and_yesterdays_daily_builds_a_streak() {
    let mut h = title();
    let yesterday = Challenge::on(Date::new(2026, 10, 15), ID, Period::Daily);
    Challenges::submit(&mut h.app.ctx, &yesterday, 300);
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    Challenges::submit(&mut h.app.ctx, &ch, 900);
    challenges(&mut h);
    assert_said(&h, "Done, best 900, a 2 day streak.");
    brief(&mut h);
    play_nothing(&mut h);
    let summary = h.spoken()[0].clone();
    assert!(
        summary.starts_with("Show not cleared, 0 percent, 60 needed."),
        "{summary}"
    );
    assert!(
        summary.ends_with("60 needed. Your best is 900. A 2 day streak."),
        "{summary}"
    );
    assert!(played(&h, "liberty-lights/music/lose"));
    assert_eq!(Challenges::best(&mut h.app.ctx, &ch), Some(900));
}

#[test]
fn the_weekly_gauntlet_adds_up_and_ends_at_its_first_failure() {
    let mut h = title();
    challenges(&mut h);
    h.press(Key::Down);
    let weekly = plan(&h, Period::Weekly);
    h.clear();
    h.press(Key::Return);
    assert_said(&h, "The five-show gauntlet: Five shows in a row");
    h.clear();
    h.press(Key::Return);
    assert_said(&h, "Weekly challenge, show 1 of 5: The warm-up.");
    play_perfect(&mut h, &weekly.acts[0].show);
    let summary = h.spoken()[0].clone();
    assert!(
        summary.starts_with("The warm-up cleared, 100 percent, "),
        "{summary}"
    );
    assert!(summary.contains("Challenge total "), "{summary}");
    assert!(results_pages(&mut h).contains("Next: show 2 of 5"));
    goto(&mut h, "Next");
    let ch = Challenge::current(&h.app.ctx, ID, Period::Weekly);
    assert!(!Challenges::done(&mut h.app.ctx, &ch), "not over yet");
    h.clear();
    h.press(Key::Return); // Next
    assert_eq!(h.app.top_name(), "liberty_lights_show");
    assert_said(&h, "Weekly challenge, show 2 of 5: The build.");
    assert_said(&h, "show points so far.");
    play_nothing(&mut h);
    let summary = h.spoken()[0].clone();
    assert!(
        summary.contains("The gauntlet ends after 2 of 5 shows."),
        "{summary}"
    );
    let best = Challenges::best(&mut h.app.ctx, &ch).expect("submitted");
    assert!(best >= 1000, "the first show's points count: {best}");
    assert!(summary.contains(&format!("Weekly challenge result: {best} show points.")));
    assert!(summary.contains("A 1 week streak."), "{summary}");
    assert!(results_pages(&mut h).contains("A new one in 3 days."));
}

#[test]
fn the_monthly_festival_pays_for_its_signature_and_leaving_does_not_count() {
    let mut h = title();
    challenges(&mut h);
    h.press(Key::Down);
    h.press(Key::Down);
    let monthly = plan(&h, Period::Monthly);
    let fest = monthly.festival.unwrap();
    assert_eq!(
        (fest.name, fest.signature),
        ("Bonfire Parade", Kind::Crackle)
    );
    h.clear();
    h.press(Key::Return);
    assert_said(&h, "The Bonfire Parade festival:");
    assert_said(&h, "adds 100 bonus points");
    h.press(Key::Return);
    assert_said(&h, "Monthly challenge, show 1 of 3: The opening act.");
    assert_said(&h, "Crackles pay a bonus.");
    let show = &monthly.acts[0].show;
    let crackles = show.iter().filter(|l| l.kind == Kind::Crackle).count();
    play_perfect(&mut h, show);
    let summary = results_pages(&mut h);
    if crackles > 0 {
        assert!(
            summary.contains(&format!(
                "Bonfire Parade {} bonus points for your crackles.",
                crackles * 100
            )),
            "{summary}"
        );
    }
    assert!(
        summary.contains("Next: show 2 of 3, the second act."),
        "{summary}"
    );
    // Leave between acts: nothing is submitted.
    h.press(Key::End);
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_challenges");
    let ch = Challenge::current(&h.app.ctx, ID, Period::Monthly);
    assert!(!Challenges::done(&mut h.app.ctx, &ch));
}

#[test]
fn a_festival_plays_every_act_even_after_a_failure() {
    let mut h = title();
    challenges(&mut h);
    h.press(Key::End);
    h.press(Key::Up);
    brief(&mut h);
    for act in 1..=3 {
        assert_said(&h, &format!("show {act} of 3"));
        play_nothing(&mut h);
        if act < 3 {
            h.clear();
            h.press(Key::Return); // Next
        }
    }
    let summary = h.spoken()[0].clone();
    assert!(
        summary.contains("Monthly challenge result: 0 show points."),
        "{summary}"
    );
    let ch = Challenge::current(&h.app.ctx, ID, Period::Monthly);
    assert_eq!(Challenges::best(&mut h.app.ctx, &ch), Some(0));
}

#[test]
fn the_launcher_opens_a_challenge_and_pausing_out_returns_there_uncounted() {
    let mut h = base();
    liberty_lights::open_challenge(&mut h.app.ctx, Period::Daily);
    h.clear();
    h.frame(0.0);
    assert_eq!(h.app.top_name(), "pages");
    assert_said(&h, "Tonight's show:");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_show");
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "pause");
    goto(&mut h, "Leave the challenge");
    h.press(Key::Return);
    h.frame(0.0);
    assert_eq!((h.app.top_name(), h.app.depth()), ("base", 1));
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(!Challenges::done(&mut h.app.ctx, &ch));
}

#[test]
fn a_challenge_from_the_launcher_returns_there_when_finished() {
    let mut h = base();
    liberty_lights::open_challenge(&mut h.app.ctx, Period::Daily);
    h.frame(0.0);
    h.press(Key::Return);
    play_nothing(&mut h);
    h.press(Key::Escape);
    h.frame(0.0);
    assert_eq!((h.app.top_name(), h.app.depth()), ("base", 1));
    let ch = Challenge::current(&h.app.ctx, ID, Period::Daily);
    assert!(Challenges::done(&mut h.app.ctx, &ch));
}

/// A one-show challenge of exactly `show` under `twist`, pushed as a show.
fn twisted(h: &mut Headless<Parlor>, twist: Twist, show: Vec<Launch>) {
    let ch = Challenge::on(TODAY, ID, Period::Daily);
    let plan = Plan {
        period: Period::Daily,
        acts: vec![Act {
            name: "Tonight's show".to_string(),
            show: show.clone(),
            placed: true,
        }],
        twist: Some(twist),
        festival: None,
        ends_on_fail: false,
    };
    let run = Run::with_plan(ch, plan);
    h.app
        .ctx
        .push(Show::with_show(Mode::Challenge(Box::new(run)), show));
    h.frame(0.0);
}

#[test]
fn one_shot_plays_the_show_once_with_no_hints() {
    let mut h = title();
    twisted(
        &mut h,
        Twist::OneShot,
        vec![Launch::new(Kind::Boom, Pos::Center, 0)],
    );
    assert_said(&h, "Twist: one shot.");
    listen(&mut h);
    h.clear();
    h.press(Key::Tab);
    assert_eq!(
        h.spoken(),
        ["Tonight's twist is one shot: the show plays only once."]
    );
    h.clear();
    h.press(Key::H);
    assert_eq!(h.spoken(), ["No hints tonight: the twist is one shot."]);
}

#[test]
fn night_fog_plays_the_show_far_off_but_your_launches_close() {
    let mut h = title();
    twisted(
        &mut h,
        Twist::NightFog,
        vec![Launch::new(Kind::Sparkle, Pos::Left, 0)],
    );
    h.sounds.clear();
    listen(&mut h);
    let far = h.sounds.calls().iter().any(|c| match c {
        Call::Play { key, mix, .. } => {
            key == "liberty-lights/fireworks/sparkle" && mix.relative.is_some_and(|r| r.x < -3.0)
        }
        _ => false,
    });
    assert!(far, "{:?}", h.sounds.calls());
    h.press(Key::Space);
    h.sounds.clear();
    h.press(Key::Num4);
    let near = h.sounds.calls().iter().any(|c| match c {
        Call::Play { key, mix, .. } => {
            key == "liberty-lights/fireworks/sparkle" && mix.relative.is_some_and(|r| r.x == 0.0)
        }
        _ => false,
    });
    assert!(near, "{:?}", h.sounds.calls());
}

#[test]
fn a_tough_crowd_pays_no_timing_for_a_late_firework() {
    let mut h = title();
    twisted(
        &mut h,
        Twist::ToughCrowd,
        vec![Launch::new(Kind::Boom, Pos::Center, 1000)],
    );
    listen(&mut h);
    // Half a second late: 15 timing points normally, none to a tough crowd.
    perform(&mut h, &[Launch::new(Kind::Boom, Pos::Center, 1500)]);
    h.clear();
    h.press(Key::Return);
    assert!(
        h.spoken()[0].starts_with("Show cleared, 75 percent, 750 points."),
        "{:?}",
        h.spoken()
    );
}

#[test]
fn a_controller_plays_a_whole_level_with_a_and_the_d_pad() {
    let mut h = title();
    h.connect_pad();
    h.press(Key::Return); // Start level 1
    assert_eq!(h.app.top_name(), "liberty_lights_show");
    let show = generate(1, &mut parlor_core::Rng::seeded(4));
    h.clear();
    // A is Enter to menus; in the show it is the main action.
    tap(&mut h, ControllerButton::A);
    assert_eq!(h.spoken(), ["The show starts."]);
    for _ in 0..1000 {
        if spoke(&h, "Your turn.") {
            break;
        }
        h.frame(0.05);
    }
    assert_said(&h, "Your turn.");
    // On a controller the turn names the wheel and how to finish with it.
    assert_said(&h, "launches the chosen firework.");
    assert_said(&h, "When finished, choose Finish with");
    tap(&mut h, ControllerButton::A); // the count-in
                                      // Choose the show's firework with the d-pad.
    for _ in 0..show[0].kind.digit() - 1 {
        tap(&mut h, ControllerButton::DPadDown);
    }
    if show[0].kind != Kind::Boom {
        assert_eq!(
            last(&h),
            format!(
                "{}, key {}.",
                hearken::text::sentence_start(show[0].kind.name()),
                show[0].kind.digit()
            )
        );
    }
    let mut ms = -1500;
    while ms < show[0].ms {
        h.frame(0.01);
        ms += 10;
    }
    h.sounds.clear();
    tap(&mut h, ControllerButton::A);
    assert!(played(
        &h,
        &format!("liberty-lights/fireworks/{}", show[0].kind.name())
    ));
    assert_eq!(
        h.app.top_name(),
        "liberty_lights_show",
        "A launched, it did not finish"
    );
    // Up from the firework steps back round to Finish.
    for _ in 0..show[0].kind.digit() {
        tap(&mut h, ControllerButton::DPadUp);
    }
    assert_eq!(last(&h), "Finish the show and score it.");
    assert!(played(&h, "liberty-lights/cue/select"));
    h.clear();
    tap(&mut h, ControllerButton::A);
    assert_eq!(h.app.top_name(), "results");
    assert!(h.spoken()[0].contains(", 100 percent,"), "{:?}", h.spoken());
}

#[test]
fn the_wheel_takes_back_and_number_keys_choose_its_firework() {
    let mut h = title();
    h.press(Key::Return);
    listen(&mut h);
    h.press(Key::Space);
    h.press(Key::Num3); // a whistle, now chosen
    h.sounds.clear();
    h.press(Key::Space);
    assert!(played(&h, "liberty-lights/fireworks/whistle"));
    h.clear();
    h.press(Key::S);
    assert!(last(&h).contains("Launched 2 of 1"), "{}", last(&h));
    assert!(
        last(&h).contains("Whistle, key 3 selected."),
        "{}",
        last(&h)
    );
    h.press(Key::Down); // sparkle
    h.clear();
    h.press(Key::Down);
    assert_eq!(h.spoken(), ["Take back the last firework."]);
    h.clear();
    h.press(Key::Space);
    assert_eq!(h.spoken(), ["Last firework removed: whistle."]);
    h.clear();
    h.press(Key::R);
    assert!(
        last(&h).contains("Space now: Take back the last firework."),
        "{}",
        last(&h)
    );
}

#[test]
fn a_controller_composes_and_finishes_a_freestyle_show() {
    let mut h = title();
    h.connect_pad();
    goto(&mut h, "Freestyle show");
    h.press(Key::Return);
    h.press(Key::Return); // Compose
    assert_eq!(h.app.top_name(), "liberty_lights_freestyle");
    h.sounds.clear();
    tap(&mut h, ControllerButton::A);
    assert!(played(&h, "liberty-lights/fireworks/boom"));
    assert_eq!(h.app.top_name(), "liberty_lights_freestyle");
    // Hold Left on the d-pad to launch on the left.
    h.send(InputEvent::button(ControllerButton::DPadLeft));
    h.sounds.clear();
    tap(&mut h, ControllerButton::A);
    h.send(InputEvent::button_up(ControllerButton::DPadLeft));
    let left = h.sounds.calls().iter().any(|c| match c {
        Call::Play { key, mix, .. } => {
            key == "liberty-lights/fireworks/boom" && mix.relative.is_some_and(|r| r.x < 0.0)
        }
        _ => false,
    });
    assert!(left, "{:?}", h.sounds.calls());
    tap(&mut h, ControllerButton::DPadUp);
    assert_eq!(last(&h), "Finish the show and score it.");
    tap(&mut h, ControllerButton::A);
    assert_eq!(h.app.top_name(), "liberty_lights_freestyle_done");
    assert_said(&h, "You launched 2 fireworks.");
}

#[test]
fn the_launch_range_launches_the_chosen_firework() {
    let mut h = title();
    h.connect_pad();
    goto(&mut h, "Practice room");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_range");
    h.press(Key::Up);
    assert_eq!(last(&h), "Sparkle, key 4.");
    tap(&mut h, ControllerButton::A);
    assert_eq!(last(&h), "Sparkle in the center.");
}

#[test]
fn the_pads_status_button_answers_in_the_range_and_a_replay() {
    // Y stands for no key, only for Status: the range and a replay must
    // still hear it.
    let mut h = base();
    h.connect_pad();
    h.app.ctx.push(liberty_lights::Range::new());
    h.frame(0.0);
    h.clear();
    tap(&mut h, ControllerButton::Y);
    assert_eq!(last(&h), "0 fireworks launched here.");
    let show = vec![Launch::new(Kind::Boom, Pos::Center, 0)];
    h.app
        .ctx
        .push(liberty_lights::Replay::show("Your show", show));
    h.frame(0.0);
    h.clear();
    tap(&mut h, ControllerButton::Y);
    assert!(last(&h).starts_with("Part 1 of 1"), "{}", last(&h));
}
