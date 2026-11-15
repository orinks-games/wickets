//! Sounds keep their cell in the pantry, so the moment the mouse steps
//! they move around it, not on the next sniff.
use hearken::{Call, Ctx, Headless, InputEvent, Key, State, Vec3};
use pantry_mouse::rules::{self, Dir};
use pantry_mouse::{sound, Mode, Play, ID};
use parlor_core::{Date, Parlor};

struct Base;
impl State<Parlor> for Base {
    fn name(&self) -> &'static str {
        "base"
    }
    fn help(&self, _: &Ctx<Parlor>) -> String {
        String::new()
    }
}

#[test]
fn a_step_re_pans_a_sniff_while_it_sounds() {
    rules::fix_seed(Some(11));
    let mut h = Headless::new(Default::default(), Base);
    h.app.ctx.game.today = Some(Date::new(2026, 10, 6));
    parlor_core::menus::mark_tutorial_read(&mut h.app.ctx, ID);
    h.app.ctx.push(Play::with_seed(Mode::Practice, 11));
    h.frame(0.0);
    h.app.ctx.stop_speech();
    h.clear();
    h.press(Key::Space);
    let (channel, before) = h
        .sounds
        .calls()
        .into_iter()
        .find_map(|c| match c {
            Call::Play {
                channel, key, mix, ..
            } if key == sound::SNIFF_TICK => Some((channel, mix.relative.expect("a placed sound"))),
            _ => None,
        })
        .expect("a sniff ticked");
    // Try each way until one is open: a bump leaves the mouse where it is.
    for dir in [Dir::Up, Dir::Left, Dir::Down, Dir::Right] {
        let key = match dir {
            Dir::Up => Key::Up,
            Dir::Left => Key::Left,
            Dir::Down => Key::Down,
            Dir::Right => Key::Right,
        };
        h.clear();
        h.press(key);
        h.send(InputEvent::key_up(key));
        h.frame(1.0 / 60.0);
        if !h.played().iter().any(|k| k == sound::STEP) {
            continue;
        }
        let after = h
            .sounds
            .calls()
            .into_iter()
            .find_map(|c| match c {
                Call::SetMix {
                    channel: ch, mix, ..
                } if ch == channel => mix.relative,
                _ => None,
            })
            .unwrap_or_else(|| panic!("the sniff was not re-panned: {:?}", h.sounds.calls()));
        let (dx, dy) = dir.delta();
        let step: Vec3 = sound::place(dx, dy);
        assert!(
            (after - (before - step)).length() < 0.01,
            "{after:?} vs {before:?} less {step:?}"
        );
        return;
    }
    panic!("the mouse could not step");
}
