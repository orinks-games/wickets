//! Liberty Lights driven the way a player drives it, by keys and frames.

use hearken::{Ctx, Headless, InputEvent, Key, State};
use liberty_lights::progress::{self, Progress};
use liberty_lights::rules::{self, generate, generate_finale, Kind, Launch, Pos};
use liberty_lights::{title, ID};
use parlor_core::menus;
use parlor_core::{Parlor, Records, Rng};

const SEED: u64 = 4;

/// A screen standing in for the launcher, so leaving the game has
/// somewhere to go.
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

/// The title over the launcher's stand-in, with the tutorial already
/// read so the cursor starts on Play, and nothing spoken yet but the title.
fn start() -> Headless<Parlor> {
    from_base()
}

fn from_base() -> Headless<Parlor> {
    rules::fix_seed(Some(SEED));
    let mut h = Headless::new(Default::default(), Base);
    // A date far from the Fourth, so Independence night is locked.
    h.app.ctx.game.today = Some(parlor_core::Date::new(2026, 10, 16));
    menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h.clear();
    h.app.ctx.push(title());
    h.frame(0.0);
    h
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

fn spoke(h: &Headless<Parlor>, part: &str) -> bool {
    h.spoken().iter().any(|l| l.contains(part))
}

fn last(h: &Headless<Parlor>) -> String {
    h.spoken().last().cloned().unwrap_or_default()
}

fn played(h: &Headless<Parlor>, key: &str) -> bool {
    h.played().iter().any(|k| k == key)
}

fn level_show(level: u32) -> Vec<Launch> {
    generate(level, &mut Rng::seeded(SEED))
}

/// From the show's Ready phase: hear it through to "your turn".
fn listen(h: &mut Headless<Parlor>) {
    h.press(Key::Space);
    for _ in 0..1000 {
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

/// Launches one firework, holding the arrow its place needs.
fn launch(h: &mut Headless<Parlor>, kind: Kind, pos: Pos) {
    let hold = match pos {
        Pos::Left => Some(Key::Left),
        Pos::Right => Some(Key::Right),
        Pos::Center => None,
    };
    if let Some(k) = hold {
        h.send(InputEvent::key(k));
    }
    h.press(digit(kind));
    if let Some(k) = hold {
        h.send(InputEvent::key_up(k));
    }
}

/// From "your turn": counts in and launches `launches` on time, without
/// pressing Enter.
fn perform(h: &mut Headless<Parlor>, launches: &[Launch]) {
    h.press(Key::Space);
    let mut ms = -1500;
    for l in launches {
        while ms < l.ms {
            h.frame(0.01);
            ms += 10;
        }
        launch(h, l.kind, l.pos);
    }
}

/// Plays the title's first row, the current level, through to the results.
fn play_level_from_title(h: &mut Headless<Parlor>, launches: impl Fn(&[Launch]) -> Vec<Launch>) {
    let level = Progress::load(&mut h.app.ctx).next_level();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_show");
    listen(h);
    let show = level_show(level);
    perform(h, &launches(&show));
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "results");
}

/// Opens the level select and starts `level`.
fn select_level(h: &mut Headless<Parlor>, level: u32) {
    goto(h, "Level select");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_levels");
    h.press(Key::Home);
    for _ in 1..level {
        h.press(Key::Down);
    }
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_show");
}

#[test]
fn the_title_announces_itself() {
    let h = start();
    assert_eq!(h.spoken(), ["Liberty Lights.", "Play, level 1. 1 of 13."]);
    assert!(played(&h, "liberty-lights/music/title"));
}

#[test]
fn starting_a_level_speaks_its_intro_and_quiets_the_music() {
    let mut h = start();
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_show");
    assert_eq!(
        h.spoken(),
        ["Level 1. Single firework timing. Hear the show, then launch it back. Space plays the show."]
    );
    assert_eq!(h.app.ctx.audio.music(), None);
}

#[test]
fn the_show_counts_in_then_plays_its_fireworks() {
    let mut h = start();
    h.press(Key::Return);
    h.clear();
    listen(&mut h);
    let ticks = h
        .played()
        .iter()
        .filter(|k| *k == "liberty-lights/count/tick")
        .count();
    assert_eq!(ticks, 3);
    assert!(played(&h, "liberty-lights/count/go"));
    let kind = level_show(1)[0].kind;
    assert!(played(
        &h,
        &format!("liberty-lights/fireworks/{}", kind.name())
    ));
    assert!(played(&h, "liberty-lights/launch/fuse"));
    assert_eq!(h.spoken()[0], "Listen.");
}

#[test]
fn space_while_the_show_plays_asks_you_to_wait() {
    let mut h = start();
    h.press(Key::Return);
    h.press(Key::Space);
    h.frame(0.1);
    h.clear();
    h.press(Key::Space);
    assert_eq!(h.spoken(), ["The show is still playing."]);
    h.clear();
    h.press(Key::Num1);
    assert_eq!(h.spoken(), ["Wait for the show to finish."]);
}

#[test]
fn a_perfect_level_one_clears_saves_a_best_and_moves_on() {
    let mut h = start();
    play_level_from_title(&mut h, |show| show.to_vec());
    let summary = h.spoken()[0].clone();
    assert_eq!(
        summary,
        "Level 1 cleared, 100 percent, 1000 points. First score saved."
    );
    assert!(played(&h, "liberty-lights/crowd/cheer"));
    assert!(played(&h, "liberty-lights/music/win"));
    assert_eq!(h.spoken()[1], "Next level. 1 of 5.");
    let ctx = &mut h.app.ctx;
    assert_eq!(Records::best(ctx, ID, "level 1"), Some(1000));
    assert_eq!(Records::best(ctx, ID, progress::TOTAL_BOARD), Some(1000));
    let p = Progress::load(ctx);
    assert_eq!(p.next_level(), 2);
    assert!(p.is_cleared(1));
    // Escape goes back to the title, which offers level 2 now.
    h.clear();
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "liberty_lights");
    assert_eq!(h.spoken(), ["Liberty Lights.", "Play, level 2. 1 of 13."]);
}

#[test]
fn launching_nothing_fails_with_a_confused_crowd() {
    let mut h = start();
    play_level_from_title(&mut h, |_| Vec::new());
    let summary = h.spoken()[0].clone();
    assert!(
        summary.starts_with("Level 1 not cleared, 0 percent, 60 needed."),
        "{summary}"
    );
    assert!(played(&h, "liberty-lights/crowd/murmur"));
    assert!(played(&h, "liberty-lights/music/lose"));
    assert_eq!(h.spoken()[1], "Try again. 1 of 5.");
    // The points the level keeps, and what the missing firework cost.
    goto(&mut h, "Hear the results again");
    h.press(Key::Return);
    h.clear();
    h.press(Key::Return);
    assert_eq!(
        h.spoken(),
        ["0 show points. The crowd murmurs, confused. You launched 0 of 1."]
    );
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.spoken(), ["Firework 1 scored 0. 1 too few, 20 off."]);
    h.press(Key::Return);
    let p = Progress::load(&mut h.app.ctx);
    assert_eq!(p.next_level(), 1);
    assert!(!p.is_cleared(1));
}

#[test]
fn the_wrong_firework_on_time_scores_half_and_the_crowd_aahs() {
    let mut h = start();
    play_level_from_title(&mut h, |show| {
        let mut l = show[0];
        l.kind = Kind::from_index(l.kind as u32 + 1);
        vec![l]
    });
    let summary = h.spoken()[0].clone();
    assert!(
        summary.starts_with("Level 1 not cleared, 50 percent, 60 needed."),
        "{summary}"
    );
    assert!(played(&h, "liberty-lights/crowd/aah"));
}

#[test]
fn being_late_costs_timing_points_by_the_original_windows() {
    let mut h = start();
    // The right firework, 400 ms late: 50 + 25 + 15.
    play_level_from_title(&mut h, |show| {
        let mut l = show[0];
        l.ms += 400;
        vec![l]
    });
    assert!(h.spoken()[0].contains("cleared, 90 percent,"));
}

#[test]
fn an_extra_firework_costs_twenty() {
    let mut h = start();
    play_level_from_title(&mut h, |show| {
        let mut v = show.to_vec();
        v.push(Launch::new(Kind::Sparkle, Pos::Center, show[0].ms + 1000));
        v
    });
    assert!(h.spoken()[0].contains(", 80 percent,"), "{:?}", h.spoken());
    goto(&mut h, "Hear the results again");
    h.press(Key::Return);
    h.clear();
    h.press(Key::Return);
    assert!(last(&h).contains("You launched 2 of 1."), "{}", last(&h));
}

#[test]
fn practice_this_level_replays_the_same_pattern() {
    let mut h = start();
    play_level_from_title(&mut h, |_| Vec::new());
    goto(&mut h, "Practice this level");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_show");
    listen(&mut h);
    perform(&mut h, &level_show(1));
    h.clear();
    h.press(Key::Return);
    assert!(h.spoken()[0].contains(", 100 percent,"));
}

#[test]
fn next_level_carries_the_run_total() {
    let mut h = start();
    play_level_from_title(&mut h, |show| show.to_vec());
    h.clear();
    h.press(Key::Return); // Next level
    assert_eq!(h.app.top_name(), "liberty_lights_show");
    assert!(h.spoken()[0].starts_with("Level 2."));
    h.clear();
    h.press(Key::S);
    h.press(Key::S);
    assert!(
        last(&h).contains("1000 show points so far."),
        "{}",
        last(&h)
    );
    listen(&mut h);
    // After level 1 the turn line is short.
    assert_eq!(last(&h), "Your turn. Space counts in.");
    perform(&mut h, &level_show(2));
    h.clear();
    h.press(Key::Return);
    goto(&mut h, "Hear the results again");
    h.press(Key::Return);
    h.clear();
    h.press(Key::Return);
    assert!(last(&h).contains("Run total 2000."), "{}", last(&h));
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, progress::TOTAL_BOARD),
        Some(2000)
    );
}

#[test]
fn placed_levels_need_the_right_side_and_build_combos() {
    let mut h = start();
    select_level(&mut h, 11);
    assert!(h.spoken()[0].starts_with("Level 11. Adds left, center, and right launch positions."));
    let show = level_show(11);
    assert!(show.iter().any(|l| l.pos != Pos::Center), "{show:?}");
    listen(&mut h);
    assert!(
        last(&h).contains("Hold the Left arrow or A"),
        "{}",
        last(&h)
    );
    perform(&mut h, &show);
    h.clear();
    h.press(Key::Return);
    let summary = h.spoken()[0].clone();
    assert!(summary.contains(", 100 percent, 1150 points."), "{summary}");
    assert!(played(&h, "liberty-lights/cue/combo"));
    // Three perfect fireworks in a row: a combo of 3, 50 + 100 bonus.
    goto(&mut h, "Hear the results again");
    h.press(Key::Return);
    h.clear();
    h.press(Key::Return);
    assert!(
        last(&h).contains("Best combo 3, 150 bonus points."),
        "{}",
        last(&h)
    );
    assert_eq!(Progress::load(&mut h.app.ctx).best_combo, 3);
}

#[test]
fn the_wrong_side_loses_a_quarter_of_each_firework() {
    let mut h = start();
    select_level(&mut h, 11);
    listen(&mut h);
    let mirrored: Vec<Launch> = level_show(11)
        .into_iter()
        .map(|mut l| {
            l.pos = match l.pos {
                Pos::Left => Pos::Right,
                Pos::Right => Pos::Left,
                Pos::Center => Pos::Left,
            };
            l
        })
        .collect();
    perform(&mut h, &mirrored);
    h.clear();
    h.press(Key::Return);
    assert!(
        h.spoken()[0].starts_with("Level 11 cleared, 75 percent,"),
        "{:?}",
        h.spoken()
    );
}

#[test]
fn level_20_plays_every_firework_and_opens_the_finale() {
    let mut h = start();
    select_level(&mut h, 20);
    let show = level_show(20);
    assert_eq!(show.len(), 8);
    h.clear();
    listen(&mut h);
    let bursts = h
        .played()
        .iter()
        .filter(|k| k.starts_with("liberty-lights/fireworks/"))
        .count();
    assert_eq!(bursts, 8);
    perform(&mut h, &show);
    h.clear();
    h.press(Key::Return);
    let summary = h.spoken()[0].clone();
    assert!(summary.contains("All 20 levels cleared, "), "{summary}");
    assert!(summary.contains("The grand finale is open"));
    assert!(played(&h, "liberty-lights/music/finale"));
    assert!(Progress::load(&mut h.app.ctx).finale_open());
    assert_eq!(h.spoken()[1], "Play again. 1 of 4.");
    goto(&mut h, "Back to the title");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights");
    h.press(Key::Home);
    h.press(Key::Down);
    assert_eq!(last(&h), "Grand finale. 2 of 13.");
}

#[test]
fn the_grand_finale_is_locked_until_level_20() {
    let mut h = start();
    h.press(Key::Down);
    assert_eq!(last(&h), "Grand finale, locked. 2 of 13.");
    h.clear();
    h.press(Key::Return);
    assert_eq!(
        h.spoken(),
        ["The grand finale opens once you clear level 20."]
    );
    assert_eq!(h.app.top_name(), "liberty_lights");
}

#[test]
fn the_grand_finale_runs_until_a_round_fails() {
    let mut h = start();
    let mut p = Progress::default();
    p.clear(20);
    p.store(&mut h.app.ctx);
    h.app.ctx.push(title());
    h.frame(0.0);
    goto(&mut h, "Grand finale");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_show");
    assert!(
        h.spoken()[0].starts_with("Grand finale, round 1. 6 fireworks"),
        "{:?}",
        h.spoken()
    );
    let round1 = generate_finale(1, &mut Rng::seeded(SEED));
    listen(&mut h);
    perform(&mut h, &round1);
    h.clear();
    h.press(Key::Return);
    assert!(
        h.spoken()[0].contains("Round 1 cleared, "),
        "{:?}",
        h.spoken()
    );
    assert_eq!(h.spoken()[1], "Next round. 1 of 4.");
    h.clear();
    h.press(Key::Return);
    assert!(h.spoken()[0].starts_with("Grand finale, round 2."));
    listen(&mut h);
    h.press(Key::Space);
    h.clear();
    h.press(Key::Return);
    let summary = h.spoken()[0].clone();
    assert!(
        summary.contains("The finale ends after 1 round cleared, "),
        "{summary}"
    );
    let best = Records::best(&mut h.app.ctx, ID, progress::FINALE_BOARD).unwrap();
    assert!(best >= 1000, "{best}");
}

#[test]
fn backspace_takes_back_the_last_firework() {
    let mut h = start();
    h.press(Key::Return);
    listen(&mut h);
    h.press(Key::Space);
    h.clear();
    h.press(Key::Backspace);
    assert_eq!(h.spoken(), ["Nothing to take back."]);
    h.press(Key::Num1);
    h.clear();
    h.press(Key::Backspace);
    assert_eq!(h.spoken(), ["Last firework removed: boom."]);
    assert!(played(&h, "liberty-lights/cue/undo"));
    h.clear();
    h.press(Key::S);
    assert!(last(&h).contains("Launched 0 of 1"), "{}", last(&h));
}

#[test]
fn launching_out_of_turn_is_explained() {
    let mut h = start();
    h.press(Key::Return);
    h.clear();
    h.press(Key::Num2);
    assert_eq!(h.spoken(), ["Listen first: press Space to hear the show."]);
    listen(&mut h);
    h.clear();
    h.press(Key::Num2);
    assert_eq!(
        h.spoken(),
        ["Press Space for the count-in first, then launch."]
    );
    h.clear();
    h.press(Key::Return);
    assert_eq!(
        h.spoken(),
        ["Press Space for the count-in and launch your show first."]
    );
}

#[test]
fn a_hint_describes_the_show_and_costs_fifteen_points() {
    let mut h = start();
    h.press(Key::Return);
    h.clear();
    h.press(Key::H);
    assert_eq!(h.spoken(), ["Hear the show first: press Space."]);
    listen(&mut h);
    h.clear();
    h.press(Key::H);
    let show = level_show(1);
    assert_eq!(
        last(&h),
        format!("{} That hint costs 15 points.", rules::describe_show(&show))
    );
    assert!(played(&h, "liberty-lights/cue/hint"));
    perform(&mut h, &show);
    h.clear();
    h.press(Key::Return);
    assert!(h.spoken()[0].contains(", 85 percent,"));
    goto(&mut h, "Hear the results again");
    h.press(Key::Return);
    h.clear();
    h.press(Key::Return);
    assert!(last(&h).contains("Hint used."), "{}", last(&h));
}

#[test]
fn tab_hears_the_show_again() {
    let mut h = start();
    h.press(Key::Return);
    listen(&mut h);
    h.press(Key::Space);
    h.press(Key::Num1);
    h.clear();
    h.press(Key::Tab);
    assert_eq!(h.spoken(), ["Listen."]);
    for _ in 0..200 {
        h.frame(0.05);
    }
    assert!(spoke(&h, "Your turn."));
    h.clear();
    h.press(Key::S);
    assert!(last(&h).contains("Your turn"), "{}", last(&h));
}

#[test]
fn repeat_status_and_help_speak_for_each_phase() {
    let mut h = start();
    h.press(Key::Return);
    h.clear();
    h.press(Key::R);
    assert_eq!(
        h.spoken(),
        ["Listen phase. Press Space to hear the show, then launch it back."]
    );
    h.clear();
    h.press(Key::S);
    assert!(
        last(&h).starts_with("Level 1 of 20, 1 firework."),
        "{}",
        last(&h)
    );
    h.clear();
    h.press(Key::F1);
    assert_eq!(
        last(&h),
        "Level 1. Space plays the show. F1 again for every key."
    );
    h.clear();
    h.press(Key::F1);
    assert!(last(&h).contains("Escape pauses."));
    assert!(last(&h).contains("Every firework in this level goes up in the center."));
    h.clear();
    h.press(Key::Q);
    assert!(h.spoken().is_empty(), "{:?}", h.spoken());
    listen(&mut h);
    h.press(Key::Space);
    h.clear();
    h.press(Key::R);
    assert!(last(&h).starts_with("Play phase. Keys 1 through 4 launch fireworks"));
}

#[test]
fn aiming_ticks_on_that_side() {
    let mut h = start();
    h.press(Key::Return);
    h.clear();
    h.send(InputEvent::key(Key::Left));
    let calls = h.sounds.calls();
    let left = calls.iter().any(|c| match c {
        hearken::Call::Play { key, mix, .. } => {
            key == "liberty-lights/cue/aim" && mix.relative.is_some_and(|r| r.x < 0.0)
        }
        _ => false,
    });
    assert!(left, "{calls:?}");
}

#[test]
fn escape_pauses_and_the_pause_menu_returns_to_the_title() {
    let mut h = start();
    h.press(Key::Return);
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "pause");
    h.clear();
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "liberty_lights_show");
    assert!(h.spoken()[0].starts_with("Level 1. Listen phase."));
    h.press(Key::Escape);
    assert_eq!(
        menus::rows_of(&h.app.top_lines()),
        [
            "Resume",
            "Settings",
            "Help",
            "Back to the title",
            "Back to the lobby"
        ]
    );
    goto(&mut h, "Back to the title");
    h.clear();
    h.press(Key::Return);
    assert_eq!((h.app.top_name(), h.app.depth()), ("liberty_lights", 2));
    assert_eq!(h.spoken()[0], "Liberty Lights.");
}

#[test]
fn compare_plays_the_show_then_yours() {
    let mut h = start();
    play_level_from_title(&mut h, |show| show.to_vec());
    goto(&mut h, "Compare");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_replay");
    assert_eq!(h.spoken(), ["The show."]);
    for _ in 0..400 {
        h.frame(0.05);
        if h.app.top_name() == "results" {
            break;
        }
    }
    assert_eq!(h.app.top_name(), "results");
    assert!(spoke(&h, "Yours."));
    assert!(spoke(&h, "That's the show."));
}

#[test]
fn back_to_the_parlor_leaves_the_game() {
    let mut h = from_base();
    h.press(Key::End);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "base");
    let mut h = from_base();
    h.press(Key::Return);
    listen(&mut h);
    h.press(Key::Space);
    h.press(Key::Return); // score nothing
    assert_eq!(
        menus::rows_of(&h.app.top_lines()),
        [
            "Try again",
            "Hear the results again",
            "Practice this level",
            "Compare",
            "Back to the title"
        ]
    );
    h.press(Key::End);
    h.press(Key::Return);
    assert_eq!((h.app.top_name(), h.app.depth()), ("liberty_lights", 2));
    h.press(Key::End);
    h.press(Key::Return);
    assert_eq!((h.app.top_name(), h.app.depth()), ("base", 1));
}

#[test]
fn results_back_row_matches_escape() {
    let mut by_row = from_base();
    play_level_from_title(&mut by_row, |show| show.to_vec());
    by_row.press(Key::End);
    by_row.press(Key::Return);
    let mut by_escape = from_base();
    play_level_from_title(&mut by_escape, |show| show.to_vec());
    by_escape.press(Key::Escape);
    assert_eq!(
        (by_row.app.top_name(), by_row.app.depth()),
        (by_escape.app.top_name(), by_escape.app.depth())
    );
    assert_eq!(by_row.app.top_name(), "liberty_lights");
}

#[test]
fn freestyle_composes_saves_and_challenges_your_show() {
    let mut h = start();
    goto(&mut h, "Freestyle show");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_freestyle_menu");
    assert_eq!(last(&h), "Compose a new show. 1 of 3.");
    h.press(Key::Down);
    assert_eq!(last(&h), "No saved show yet. 2 of 3.");
    h.press(Key::Up);
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_freestyle");
    assert!(h.spoken()[0].starts_with("Freestyle show. Compose your own show."));
    h.frame(3.0);
    launch(&mut h, Kind::Boom, Pos::Left);
    h.frame(0.5);
    launch(&mut h, Kind::Crackle, Pos::Right);
    h.frame(0.5);
    launch(&mut h, Kind::Whistle, Pos::Center);
    h.clear();
    h.press(Key::S);
    assert_eq!(
        h.spoken(),
        ["3 of 64 fireworks launched, 1 second from first to last."]
    );
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_freestyle_done");
    assert_eq!(
        h.spoken(),
        [
            "Freestyle show complete. You launched 3 fireworks.",
            "Hear your show. 1 of 4."
        ]
    );
    h.press(Key::Down);
    h.clear();
    h.press(Key::Return); // Save this show
    assert_eq!(
        h.spoken(),
        ["Show saved. Match it from the freestyle menu."]
    );
    assert!(played(&h, "liberty-lights/cue/saved"));
    let saved = Progress::load(&mut h.app.ctx).saved_show;
    assert_eq!(
        saved,
        [
            Launch::new(Kind::Boom, Pos::Left, 0),
            Launch::new(Kind::Crackle, Pos::Right, 500),
            Launch::new(Kind::Whistle, Pos::Center, 1000),
        ]
    );
    h.press(Key::Up);
    h.press(Key::Down);
    assert_eq!(last(&h), "Saved. 2 of 4.");
    // Escape goes back to the title; the freestyle menu offers the saved show.
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "liberty_lights");
    goto(&mut h, "Freestyle show");
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_freestyle_menu");
    h.press(Key::Down);
    assert_eq!(
        last(&h),
        "Hear your saved show, 3 fireworks, 1 second. 2 of 4."
    );
    h.press(Key::Down);
    h.clear();
    h.press(Key::Return); // Match your saved show
    assert_eq!(h.app.top_name(), "liberty_lights_show");
    assert!(h.spoken()[0].starts_with("Your saved show, 3 fireworks."));
    listen(&mut h);
    perform(&mut h, &saved);
    h.clear();
    h.press(Key::Return);
    assert!(h.spoken()[0].contains("Your show matched, "));
    assert_eq!(
        Records::best(&mut h.app.ctx, ID, progress::MY_SHOW_BOARD),
        Some(1150)
    );
}

#[test]
fn hearing_a_freestyle_show_plays_it_back_and_the_crowd_cheers() {
    let mut h = start();
    goto(&mut h, "Freestyle show");
    h.press(Key::Return);
    h.press(Key::Return); // Compose
    launch(&mut h, Kind::Sparkle, Pos::Center);
    h.press(Key::Return);
    h.clear();
    h.press(Key::Return); // Hear your show
    assert_eq!(h.app.top_name(), "liberty_lights_replay");
    for _ in 0..200 {
        h.frame(0.05);
    }
    assert_eq!(h.app.top_name(), "liberty_lights_freestyle_done");
    assert!(played(&h, "liberty-lights/fireworks/sparkle"));
    assert!(played(&h, "liberty-lights/crowd/cheer"));
    assert!(spoke(&h, "That's the show."));
}

#[test]
fn the_sound_check_names_every_cue() {
    let mut h = start();
    goto(&mut h, "Sound check");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_sounds");
    assert_eq!(h.spoken(), ["Sound check.", "Boom, key 1. 1 of 18."]);
    h.clear();
    h.press(Key::Return);
    assert_eq!(
        h.played(),
        [
            "liberty-lights/launch/whoosh",
            "liberty-lights/fireworks/boom"
        ]
    );
    h.clear();
    h.press(Key::Right);
    assert_eq!(h.spoken(), ["Boom on the right."]);
    h.press(Key::Left);
    assert_eq!(last(&h), "Boom on the left.");
    let mut keys = std::collections::BTreeSet::new();
    // Every sound row: 17 sounds, then Back.
    for _ in 1..17 {
        h.press(Key::Down);
        h.sounds.clear();
        h.press(Key::Return);
        keys.extend(h.played());
    }
    for k in [
        "liberty-lights/fireworks/crackle",
        "liberty-lights/fireworks/whistle",
        "liberty-lights/fireworks/sparkle",
        "liberty-lights/cue/aim",
        "liberty-lights/launch/whoosh",
        "liberty-lights/count/tick",
        "liberty-lights/count/go",
        "liberty-lights/launch/fuse",
        "liberty-lights/crowd/cheer",
        "liberty-lights/crowd/ooh",
        "liberty-lights/crowd/aah",
        "liberty-lights/crowd/murmur",
        "liberty-lights/cue/combo",
        "liberty-lights/cue/undo",
        "liberty-lights/cue/select",
    ] {
        assert!(keys.contains(k), "{k} missing from {keys:?}");
    }
    h.press(Key::End);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights");
}

#[test]
fn the_practice_room_is_a_launch_range_naming_each_firework() {
    let mut h = start();
    goto(&mut h, "Practice room");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights_range");
    assert!(h.spoken()[0].starts_with("Practice room, a launch range"));
    h.clear();
    launch(&mut h, Kind::Whistle, Pos::Left);
    assert_eq!(last(&h), "Whistle on the left.");
    h.press(Key::Num4);
    assert_eq!(last(&h), "Sparkle in the center.");
}

#[test]
fn the_tutorial_launches_one_boom() {
    use liberty_lights::tutorial;
    let mut h = start();
    goto(&mut h, "Tutorial");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "lesson");
    assert!(h.spoken()[0].starts_with("You are the fireworks director"));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "liberty_lights");

    let sky = tutorial::sky();
    let lesson = tutorial::lesson_on(&mut h.app.ctx, sky.clone());
    h.app.ctx.push(lesson);
    h.frame(0.0);
    h.press(Key::Space);
    for _ in 0..(60 * 5) {
        h.frame(1.0 / 60.0);
    }
    assert!(
        spoke(&h, "your turn. Press Space for your own count-in"),
        "{:?}",
        h.spoken()
    );
    // The lesson's own step speaks on the turn, not the show's line too.
    assert!(!spoke(&h, "Your turn. Press Space for your count-in"));
    h.press(Key::Return);
    assert_eq!(last(&h), "Launch the boom first, with 1 or Space.");
    h.press(Key::Space);
    // The count in, then the boom a second after the go.
    for _ in 0..(60 * 5 / 2) {
        h.frame(1.0 / 60.0);
    }
    h.press(Key::Num1);
    assert!(
        spoke(&h, "Boom launched, perfect timing."),
        "{:?}",
        h.spoken()
    );
    assert!(spoke(&h, "Tutorial done."));
    assert_eq!(h.app.top_name(), "liberty_lights");
}

#[test]
fn records_read_as_pages() {
    let mut h = start();
    goto(&mut h, "Records");
    h.clear();
    h.press(Key::Return);
    assert!(
        h.spoken()[0].starts_with("Records. Run total: no score yet."),
        "{:?}",
        h.spoken()
    );
    h.press(Key::Return);
    assert!(last(&h).starts_with("Level 1: not cleared, no score yet."));
}

#[test]
fn the_level_select_opens_on_the_current_level_and_shows_bests() {
    let mut h = start();
    play_level_from_title(&mut h, |show| show.to_vec());
    h.press(Key::Escape);
    goto(&mut h, "Level select");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.spoken(), ["Level select.", "Level 2. 2 of 21."]);
    h.press(Key::Up);
    assert_eq!(last(&h), "Level 1, cleared, best 1000. 1 of 21.");
    h.press(Key::End);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "liberty_lights");
}

#[test]
fn a_better_score_replaces_the_best_and_a_worse_one_does_not() {
    let mut h = start();
    play_level_from_title(&mut h, |show| {
        let mut l = show[0];
        l.ms += 400;
        vec![l]
    });
    assert_eq!(Records::best(&mut h.app.ctx, ID, "level 1"), Some(900));
    h.press(Key::Down);
    h.press(Key::Return); // Play again
    listen(&mut h);
    perform(&mut h, &level_show(1));
    h.clear();
    h.press(Key::Return);
    assert!(h.spoken()[0].contains("New best, up from 900."));
    h.press(Key::Down);
    h.press(Key::Return); // Play again
    listen(&mut h);
    h.press(Key::Space);
    h.clear();
    h.press(Key::Return);
    assert!(h.spoken()[0].contains("Your best is 1000."));
}

#[test]
fn hearing_the_results_again_details_each_firework() {
    let mut h = start();
    play_level_from_title(&mut h, |show| show.to_vec());
    assert_eq!(h.spoken()[1], "Next level. 1 of 5.");
    goto(&mut h, "Hear the results again");
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "pages");
    assert!(h.spoken()[0].starts_with("Level 1 cleared, "));
    h.clear();
    h.press(Key::Return);
    assert_eq!(
        h.spoken(),
        // The first page said Enter turns pages; this one is just read.
        ["The crowd cheers. You launched 1 of 1. Run total 1000."]
    );
    h.clear();
    h.press(Key::Return);
    assert_eq!(h.spoken(), ["Firework 1 scored 100."]);
    h.press(Key::Return);
    assert_eq!(h.app.top_name(), "results");
}

#[test]
fn the_band_plays_under_the_round_and_steps_back_for_the_show() {
    use liberty_lights::show::{Mode, Show};
    use liberty_lights::sound;
    use parlor_core::score::HURRY;
    let mut h = start();
    h.press(Key::Return);
    let stems = |h: &Headless<Parlor>| {
        h.app
            .ctx
            .audio
            .live()
            .iter()
            .filter(|s| s.looping && s.bus == hearken::Bus::MUSIC && s.key.contains("/score/"))
            .count()
    };
    assert_eq!(stems(&h), sound::SCORE.len(), "every stem starts together");

    let mut show = Show::with_show(Mode::Level { level: 5, total: 0 }, level_show(5));
    let ready = show.mood();
    show.round.phase = rules::Phase::Listening;
    assert_eq!(
        show.mood().intensity,
        0.0,
        "only the base while the show plays"
    );
    show.round.phase = rules::Phase::Playing;
    let playing = show.mood();
    show.round.launches = show.round.show.clone();
    assert!(show.mood().intensity > playing.intensity && playing.intensity > ready.intensity);
    assert_eq!(show.mood().flags, 0);
    let finale = Show::with_show(Mode::Finale { round: 1, total: 0 }, level_show(5));
    assert_ne!(
        finale.mood().flags & (1 << HURRY),
        0,
        "the grand finale hurries it"
    );
}

#[test]
fn f1_starts_over_in_a_new_phase() {
    let mut h = start();
    h.press(Key::Return);
    listen(&mut h);
    h.clear();
    h.press(Key::F1);
    assert!(last(&h).starts_with("Your turn."), "{}", last(&h));
    // The count-in, then launching: nothing is said in between.
    h.press(Key::Space);
    for _ in 0..(60 * 3) {
        h.frame(1.0 / 60.0);
    }
    h.clear();
    h.press(Key::F1);
    assert!(
        last(&h).starts_with("Keys 1 through 4 launch fireworks"),
        "{}",
        last(&h)
    );
    assert!(
        last(&h).ends_with("F1 again for every key."),
        "{}",
        last(&h)
    );
}

#[test]
fn a_first_finale_ending_in_round_2_keeps_its_total_as_the_best() {
    use liberty_lights::show::{Mode, Show};
    let mut h = start();
    h.app.ctx.push(Show::with_show(
        Mode::Finale { round: 1, total: 0 },
        level_show(5),
    ));
    h.frame(0.0);
    listen(&mut h);
    perform(&mut h, &level_show(5));
    h.clear();
    h.press(Key::Return);
    let first = h.spoken()[0].clone();
    assert!(
        first.starts_with("Round 1 cleared, 100 percent, 1000 points. Finale total 1000."),
        "{first}"
    );
    goto(&mut h, "Next round");
    h.press(Key::Return);
    listen(&mut h);
    perform(&mut h, &[]);
    h.clear();
    h.press(Key::Return);
    assert_eq!(
        h.spoken()[0],
        "The finale ends after 1 round cleared, 0 percent. Finale total 1000, your best."
    );
    assert!(played(&h, "liberty-lights/music/finale"));
}
