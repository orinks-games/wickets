//! Tutorial guidance and transitions must work in both content editions.
use freewheel::{tutorial, Bike, Ride};
use hearken::{Chord, Ctx, Headless, InputEvent, Key, Mods, State};
use parlor_core::{Live, Parlor, ParlorAction};

struct Base;
impl State<Parlor> for Base {
    fn name(&self) -> &'static str {
        "test_base"
    }
    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        String::new()
    }
}

fn setup() -> (Headless<Parlor>, Vec<Live<Ride>>) {
    let mut h = Headless::new(Default::default(), Base);
    let rides: Vec<_> = (1..=5).map(|n| tutorial::ride(n, Bike::Road)).collect();
    let lesson = tutorial::lesson_on(&mut h.app.ctx, rides.clone());
    h.app.ctx.push(lesson);
    h.frame(0.0);
    (h, rides)
}

#[test]
fn completed_step_waits_for_all_held_riding_controls() {
    let (mut h, rides) = setup();
    h.press(Key::Down);
    h.press(Key::Left);
    rides[0].with_mut(|ride| ride.passed = true);
    h.clear();
    h.frame(0.0);
    assert!(h
        .spoken()
        .iter()
        .any(|s| s.contains("Release the riding controls")));
    h.clear();
    for _ in 0..3000 {
        h.send(InputEvent::KeyDown {
            key: Key::Down,
            mods: Mods::NONE,
            text: None,
            repeat: true,
        });
        h.frame(0.01);
    }
    assert!(
        h.spoken().is_empty(),
        "repeats must not overwrite the release prompt"
    );
    assert_eq!(rides[1].with(|r| r.world.clock), 0.0);
    h.send(InputEvent::key_up(Key::Down));
    h.frame(0.0);
    assert!(!h.spoken().iter().any(|s| s.contains("Passed. Brake.")));
    h.send(InputEvent::key_up(Key::Left));
    h.frame(0.0);
    assert!(h.spoken().iter().any(|s| s.contains("Passed. Brake.")));
}

#[test]
fn released_controls_advance_without_an_extra_confirmation() {
    let (mut h, rides) = setup();
    h.clear();
    rides[0].with_mut(|ride| ride.passed = true);
    h.frame(0.0);
    assert!(h.spoken().iter().any(|s| s.contains("Passed. Brake.")));
    assert!(!h
        .spoken()
        .iter()
        .any(|s| s.contains("Release the riding controls")));
}

#[test]
fn tutorial_speed_guidance_uses_rebound_status_control() {
    let mut h = Headless::new(Default::default(), Base);
    let _ = h
        .app
        .ctx
        .bindings
        .set_chord(ParlorAction::Status, Chord::plain(Key::Q));
    for n in [1, 2] {
        let page = freewheel::training::page(&h.app.ctx, n);
        assert!(page.contains("Q reports your speed"), "{page}");
    }
    assert!(freewheel::training::page(&h.app.ctx, 1).contains("tells you when to let go"));
    assert!(freewheel::training::page(&h.app.ctx, 2).contains("Wait for the instruction to brake"));
}

#[test]
fn release_gate_tracks_rebound_riding_control_and_can_be_left() {
    let (mut h, rides) = setup();
    let _ = h
        .app
        .ctx
        .bindings
        .set_chord(ParlorAction::Down, Chord::plain(Key::B));
    h.press(Key::B);
    rides[0].with_mut(|ride| ride.passed = true);
    h.clear();
    h.frame(0.0);
    assert!(h
        .spoken()
        .iter()
        .any(|s| s.contains("Release the riding controls")));
    h.press(Key::Escape);
    assert_eq!(h.app.top_name(), "test_base");
}

#[test]
fn controller_brake_release_allows_the_next_instruction() {
    let (mut h, rides) = setup();
    h.connect_pad();
    h.send(InputEvent::ControllerButtonDown {
        button: hearken::ControllerButton::DPadDown,
        instance_id: 0,
        repeat: false,
    });
    rides[0].with_mut(|ride| ride.passed = true);
    h.clear();
    h.frame(0.0);
    assert!(h
        .spoken()
        .iter()
        .any(|s| s.contains("Release the riding controls")));
    assert!(!h.spoken().iter().any(|s| s.contains("Passed. Brake.")));
    h.send(InputEvent::ControllerButtonUp {
        button: hearken::ControllerButton::DPadDown,
        instance_id: 0,
    });
    h.frame(0.0);
    assert!(h.spoken().iter().any(|s| s.contains("Passed. Brake.")));
}
