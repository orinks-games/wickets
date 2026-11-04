use super::*;
#[allow(unused_imports)]
use parlor_core::visuals::*;

impl Haunt {
    pub(super) fn visual_scene(&self, ctx: &Ctx<Parlor>) -> Scene {
        let mut s = stage(self.lines(ctx).into_iter().take(2).collect(), [18, 30, 40]);
        let r = &self.run;
        let room = &r.rooms[r.room];
        let (cw, ch) = board(&mut s, room.w as usize, room.h as usize);
        for item in &room.items {
            if !item.taken
                && (!room.dark
                    || ((item.x - r.x) as f32).hypot((item.y - r.y) as f32)
                        <= crate::rules::DARK_REACH)
            {
                marker(
                    &mut s,
                    60. + (item.x as f32 + 0.5) * cw,
                    70. + (item.y as f32 + 0.5) * ch,
                    item.item.name(),
                    GOLD,
                );
            }
        }
        player(
            &mut s,
            60. + (r.x as f32 + 0.5) * cw,
            70. + (r.y as f32 + 0.5) * ch,
        );
        s
    }
}
