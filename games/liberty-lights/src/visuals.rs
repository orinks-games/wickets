use crate::rules::{Launch, Phase, Pos, Round};
use parlor_core::visuals::*;
pub fn burst(s: &mut Scene, launch: &Launch, age: f32) {
    let x = match launch.pos {
        Pos::Left => 230.,
        Pos::Center => 500.,
        Pos::Right => 770.,
    };
    let radius = 30. + age * 120.;
    for i in 0..16 {
        let a = i as f32 * std::f32::consts::TAU / 16.;
        s.line(
            (x + a.sin() * radius * 0.5, 260. + a.cos() * radius * 0.5),
            (x + a.sin() * radius, 260. + a.cos() * radius),
            4.,
            GOLD,
        );
    }
    s.label(x - 45., 460., launch.kind.name(), PAPER);
}
pub fn draw_round(s: &mut Scene, r: &Round) {
    let launches = match r.phase {
        Phase::Listening => &r.show,
        Phase::Playing => &r.launches,
        _ => return,
    };
    for l in launches {
        let age = r.elapsed_ms() - l.ms;
        if (0..1600).contains(&age) {
            burst(s, l, age as f32 / 1600.);
        }
    }
}
