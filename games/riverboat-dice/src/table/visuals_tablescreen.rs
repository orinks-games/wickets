use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl TableScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let n = &self.table.night;
        s.rect(60., 110., 880., 380., [26, 76, 64]);
        if let Some(roll) = n.history.last() {
            for (i, value) in [roll.a, roll.b].iter().enumerate() {
                let x = 300. + i as f32 * 250.;
                s.rect(x, 200., 130., 130., PAPER);
                s.label(x + 50., 275., value.to_string(), [20, 32, 28]);
            }
        }
        for (i, seat) in n.seats.iter().enumerate() {
            marker(
                &mut s,
                lane(i, n.seats.len()),
                540.,
                seat.night.to_string(),
                if seat.seated { TEAL } else { GOLD },
            );
        }
        s
    }
}
