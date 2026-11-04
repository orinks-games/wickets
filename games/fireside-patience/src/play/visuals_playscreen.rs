use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl PlayScreen {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [18, 30, 40]);
        let t = &self.game.table;
        for (i, p) in t.piles.iter().enumerate() {
            let x = 70. + i as f32 * 125.;
            s.rect(x, 130., 100., 330., [48, 70, 65]);
            s.label(x + 8., 110., (i + 1).to_string(), PAPER);
            for (j, card) in p.up.iter().enumerate() {
                let y = 160. + j as f32 * 32.;
                s.rect(x + 4., y - 24., 92., 31., PAPER);
                s.label(
                    x + 9.,
                    y,
                    format!(
                        "{}{}",
                        match card.rank {
                            1 => "A".into(),
                            11 => "J".into(),
                            12 => "Q".into(),
                            13 => "K".into(),
                            n => n.to_string(),
                        },
                        card.suit
                            .name()
                            .chars()
                            .next()
                            .unwrap_or('?')
                            .to_ascii_uppercase()
                    ),
                    [20, 32, 28],
                );
            }
            if i == self.cursor {
                s.line((x, 480.), (x + 100., 480.), 6., GOLD);
            }
        }
        for (i, n) in t.foundations.iter().enumerate() {
            s.label(90. + i as f32 * 220., 560., n.to_string(), GOLD);
        }
        s
    }
}
