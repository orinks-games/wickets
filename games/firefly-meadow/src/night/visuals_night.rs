use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Night {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        for (i, p) in self.show.positions.iter().enumerate() {
            let lag = if self.show.out_of_step[i] {
                crate::show::LAG
            } else {
                0.
            };
            let time = self.show.time - crate::show::LEAD_IN - lag;
            let beat = (time / crate::show::BEAT).floor() as u64;
            let lit = time >= 0.
                && crate::show::Show::is_flash(beat)
                && time.rem_euclid(crate::show::BEAT) < 0.15;
            let phase = if lit { 1. } else { 0. };
            s.circle(
                500. + p.0 as f32 * 20.,
                300. + p.1 as f32 * 15.,
                5. + phase * 8.,
                GOLD,
            );
        }
        s
    }
}
