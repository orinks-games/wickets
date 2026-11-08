use super::*;
use crate::rules::BANK;
use parlor_core::visuals::*;

/// Across the ice, metres from the middle, to the screen.
fn across(x: f64) -> f32 {
    500. + (x / BANK) as f32 * 380.
}

/// How far up the screen something `ms` ahead sits.
fn up(ms: i64, lead: i64) -> f32 {
    520. - (ms.max(0) as f32 / lead.max(1) as f32).min(1.) * 420.
}

impl SkateScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [20, 30, 54]);
        let k = &self.skate;
        let now = k.now();
        let lead = k.course.level.lead_ms.max(3000);
        s.rect(80., 80., 40., 480., TEAL);
        s.rect(880., 80., 40., 480., TEAL);
        for p in &k.course.patches {
            if p.at + 400 < now || p.heard_at() > now {
                continue;
            }
            let color = match p.kind {
                crate::rules::PatchKind::Drift => PAPER,
                crate::rules::PatchKind::Thin => RED,
            };
            marker(
                &mut s,
                across(p.x),
                up(p.at - now, lead),
                p.kind.words(),
                color,
            );
        }
        for l in &k.course.lanterns {
            if l.at < now || l.lit > now {
                continue;
            }
            marker(&mut s, across(l.x()), up(l.at - now, lead), "lantern", GOLD);
        }
        if k.course.level.partner {
            marker(&mut s, across(k.partner_x()), 470., "partner", PAPER);
        }
        player(&mut s, across(k.x), 525.);
        if k.snows() {
            gauge(&mut s, 60., "Clear ice", k.clear / 100.0, k.cover().words());
        }
        s
    }
}
