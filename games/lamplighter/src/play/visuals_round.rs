use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Round {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let h = &self.hall;
        s.rect(330., 50., 340., 500., [72, 53, 38]);
        for step in h.pos.saturating_sub(3)..(h.pos + 4).min(h.length()) {
            let y = 300. + (step as f32 - h.pos as f32) * 65.;
            for (i, a) in crate::rules::Side::BOTH.iter().enumerate() {
                let x = if i == 0 { 270. } else { 730. };
                match h.alcove(step, *a) {
                    crate::rules::Alcove::Bare => {}
                    crate::rules::Alcove::Out => {
                        s.rect(x - 12., y - 12., 24., 24., [110, 120, 128]);
                    }
                    crate::rules::Alcove::Burning => {
                        s.circle(x, y, 20., GOLD);
                    }
                }
                if step == h.pos {
                    s.label(
                        x - 60.,
                        y - 30.,
                        crate::rules::alcove_words(h.alcove(step, *a)),
                        PAPER,
                    );
                }
            }
        }
        player(&mut s, 500., 300.);
        s
    }
}
