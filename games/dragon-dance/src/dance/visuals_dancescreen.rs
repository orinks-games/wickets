use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl DanceScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let d = &self.dance;
        lanes(&mut s, d.street.lanes);
        marker(&mut s, lane(d.pearl, d.street.lanes), 120., "Pearl", GOLD);
        for i in 0..d.segments.min(12) {
            s.circle(
                lane(d.lane, d.street.lanes),
                500. - i as f32 * 22.,
                18.,
                TEAL,
            );
        }
        for incoming in d.incoming() {
            let y = 500. - incoming.beats as f32 * 80.;
            if (100.0..500.0).contains(&y) {
                marker(
                    &mut s,
                    lane(incoming.lane, d.street.lanes),
                    y,
                    crate::rules::Dance::incoming_words(&incoming),
                    RED,
                );
            }
        }
        player(&mut s, lane(d.lane, d.street.lanes), 500.);
        s
    }
}
