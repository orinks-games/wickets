use super::*;
#[allow(unused_imports)]
use crate::visuals::{burst, draw_round};
use parlor_core::visuals::*;
impl Freestyle {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx), [14, 22, 45]);
        for launch in self.composer.launches.iter().rev().take(4) {
            burst(&mut s, launch, 0.6);
        }
        s
    }
}
