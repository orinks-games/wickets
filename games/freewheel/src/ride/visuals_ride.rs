use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Ride {
    pub(super) fn visual_scene(&self, _ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.status_layers(), [18, 30, 40]);
        let r = &self.world.rider;
        lanes(&mut s, 3);
        for c in &self.world.cars {
            let ahead = c.s - r.s;
            if (0.0..100.0).contains(&ahead) {
                s.rect(
                    500. + c.x as f32 * 70. - 25.,
                    500. - ahead as f32 * 4.,
                    50.,
                    65.,
                    RED,
                );
            }
        }
        let x = 500. + r.x as f32 * 70.;
        player(&mut s, x, 470. - r.z as f32 * 30.);
        s.circle(x - 18., 530., 14., TEAL);
        s.circle(x + 18., 530., 14., TEAL);
        s
    }
}
