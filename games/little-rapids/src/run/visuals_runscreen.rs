use super::*;
use crate::rules::{HALF, MILL_LEAD_BEATS};
use parlor_core::visuals::*;

/// Across the stream, metres, to the screen.
fn across(x: f64) -> f32 {
    500. + (x / HALF) as f32 * 380.
}

impl RunScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [16, 38, 52]);
        let r = &self.run;
        water(&mut s, r.time);
        // The banks.
        s.rect(80., 80., 40., 480., TEAL);
        s.rect(880., 80., 40., 480., TEAL);
        // The current, a line down the stream.
        let c = across(r.current());
        s.line((c, 120.), (c, 520.), 3., PAPER);
        let pos = r.beat_pos();
        for t in &r.things {
            let left = t.beats_left(pos).max(0.);
            let y = 520. - (left / MILL_LEAD_BEATS as f64).min(1.) as f32 * 420.;
            let color = if t.kind.is_catch() { GOLD } else { RED };
            marker(&mut s, across(t.x_at(pos)), y, t.kind.name(), color);
        }
        s.rect(across(r.x) - 30., 525., 60., 22., PAPER);
        s
    }
}
