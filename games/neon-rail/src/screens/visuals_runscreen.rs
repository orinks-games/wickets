use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl RunScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        if let Some(r) = &self.run {
            lanes(&mut s, 3);
            for e in &r.events {
                if e.announced && !e.resolved {
                    let y = 500. - ((e.resolve - r.t) * 65.) as f32;
                    if (80.0..520.0).contains(&y) {
                        let targets: &[usize] = match e.lane {
                            crate::rules::Lane::Left => &[0],
                            crate::rules::Lane::Center => &[1],
                            crate::rules::Lane::Right => &[2],
                            crate::rules::Lane::All => &[0, 1, 2],
                        };
                        for &target in targets {
                            marker(&mut s, lane(target, 3), y, r.kind_name(e.kind), RED);
                        }
                    }
                }
            }
            s.rect(lane(r.lane, 3) - 30., 490., 60., 65., TEAL);
        }
        s
    }
}
