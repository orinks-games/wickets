use super::*;
use parlor_core::visuals::*;

/// A place on the lane, to the screen.
fn across(p: Place) -> f32 {
    110. + p.index() as f32 * 130.
}

impl HamletScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        let mut s = stage(self.lines(ctx), [34, 52, 30]);
        let h = self.h.borrow();
        // The lane, with each place on it.
        s.rect(80., 300., 840., 30., PAPER);
        for p in Place::ALL {
            marker(&mut s, across(p), 270., p.name(), TEAL);
        }
        // The river runs along below.
        water(&mut s, h.hour as f64);
        // Settlers where they are working.
        for (i, st) in h.settlers.iter().filter(|s| !s.away()).enumerate() {
            let color = match st.mood() {
                Mood::Glum => RED,
                _ => GOLD,
            };
            marker(
                &mut s,
                across(st.place()),
                360. + (i % 3) as f32 * 40.,
                &st.name,
                color,
            );
        }
        player(&mut s, across(h.at.place()), 315.);
        s
    }
}
