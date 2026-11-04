use hearken::{Headless, Key};
use parlor_core::{Date, Parlor, ParlorData, Period};
use wickets::Lobby;

#[test]
fn every_games_challenge_has_a_scene_without_mutating_play() {
    let mut games = wickets::games();
    games.extend(wickets::story_games().into_iter().map(|(game, _)| game));
    games.extend(wickets::specials().into_iter().map(|(game, _)| game));
    games.sort_by_key(|g| g.id);
    games.dedup_by_key(|g| g.id);
    let mut missing = Vec::new();
    for game in games {
        let data = ParlorData {
            today: Some(Date::new(2026, 12, 15)),
            ..Default::default()
        };
        let mut h: Headless<Parlor> = Headless::new(data, Lobby::new());
        if let Some(open) = game.challenge {
            open(&mut h.app.ctx, Period::Daily);
        } else {
            (game.open)(&mut h.app.ctx);
        }
        h.frame(0.);
        let mut found = false;
        for _ in 0..40 {
            let said = h.app.ctx.said;
            let clock = h.app.ctx.clock;
            let scene = h.app.visual_scene();
            assert_eq!(scene, h.app.visual_scene(), "{}: unstable capture", game.id);
            assert_eq!(h.app.ctx.said, said, "{}: drawing spoke", game.id);
            assert_eq!(h.app.ctx.clock, clock, "{}: drawing advanced time", game.id);
            if !scene.shapes.is_empty() {
                if let Some(dir) = std::env::var_os("WICKETS_VISUAL_CAPTURE_DIR") {
                    let path = std::path::PathBuf::from(dir).join(format!("{}.bmp", game.id));
                    hearken::visuals::save_bmp(&scene, (1280, 800), &path).unwrap();
                }
                found = true;
                break;
            }
            h.press(Key::Return);
            h.frame(0.1);
        }
        if !found {
            missing.push((game.id, h.app.top_name(), h.app.top_lines()));
        }
    }
    assert!(missing.is_empty(), "No gameplay scene: {missing:#?}");
}

#[test]
fn manor_display_keeps_mimics_and_dark_room_items_hidden() {
    use haunted_hallways::{haunt::Haunt, rules::Mode};
    use hearken::State;
    let h: Headless<Parlor> = Headless::new(Default::default(), Lobby::new());
    let mut game = Haunt::with_seed(Mode::Climb(1), 31);
    let room = game.run.room;
    assert!(!game.run.rooms[room].items.is_empty());
    game.run.rooms[room].items[0].mimic = false;
    let before = game.visuals(&h.app.ctx);
    game.run.rooms[room].items[0].mimic = true;
    assert_eq!(
        before,
        game.visuals(&h.app.ctx),
        "mimic identity must not change the picture"
    );
    game.run.rooms[room].dark = true;
    game.run.rooms[room].items[0].x = game.run.x + 10;
    game.run.rooms[room].items[0].y = game.run.y + 10;
    let hidden = game.visuals(&h.app.ctx);
    game.run.rooms[room].items[0].taken = true;
    assert_eq!(
        hidden,
        game.visuals(&h.app.ctx),
        "distant dark-room items must be absent"
    );
    assert_eq!(
        hidden.lines.len(),
        2,
        "debug maps must never reach the display"
    );
}

#[test]
fn ordered_lake_shows_only_its_lit_lantern() {
    use hearken::visuals::{Shape, GOLD};
    use hearken::State;
    use lantern_lake::{
        rules::Mode,
        screens::{Launch, RunScreen},
    };
    let h: Headless<Parlor> = Headless::new(Default::default(), Lobby::new());
    let launch = (0..1000)
        .map(|seed| Launch {
            mode: Mode::Challenge(Period::Weekly),
            seed: Some(seed),
        })
        .find(|launch| launch.build().plan.ordered)
        .expect("a constellation seed");
    let game = RunScreen::new(launch, None);
    let scene = game.visuals(&h.app.ctx);
    let lights = scene
        .shapes
        .iter()
        .filter(|shape| matches!(shape,Shape::Circle{color,..} if *color==GOLD))
        .count();
    assert_eq!(lights, 1);
}

#[test]
#[cfg(windows)]
fn windows_executable_uses_the_gui_subsystem() {
    let exe = std::fs::read(env!("CARGO_BIN_EXE_wickets")).unwrap();
    let pe = u32::from_le_bytes(exe[0x3c..0x40].try_into().unwrap()) as usize;
    assert_eq!(&exe[pe..pe + 4], b"PE\0\0");
    let field = pe + 24 + 68;
    let subsystem = u16::from_le_bytes(exe[field..field + 2].try_into().unwrap());
    assert_eq!(
        subsystem, 2,
        "Wicket's must open a window without a console"
    );
}
