//! Neutral public story stub.

use hearken::{Ctx, InputEvent, Key, State};
use parlor_core::{GameInfo, Pages, Parlor};

use crate::happening::SavedHappening;

pub fn unlocked_games(_ctx: &mut Ctx<Parlor>) -> Vec<GameInfo> {
    Vec::new()
}

pub fn open(ctx: &mut Ctx<Parlor>) {
    ctx.push(Pages::new(vec!["Stub chapter 1.".to_string()]));
}

pub fn happening_now(_ctx: &mut Ctx<Parlor>) -> Option<SavedHappening> {
    None
}

pub fn errand_waits(_ctx: &mut Ctx<Parlor>) -> bool {
    false
}

pub fn open_at_happening(ctx: &mut Ctx<Parlor>) {
    open(ctx);
}

pub fn row_label() -> &'static str {
    "Story"
}

pub fn row_help(_ctx: &mut Ctx<Parlor>) -> String {
    "Stub chapter 1.".to_string()
}

pub struct SittingMenu;

impl SittingMenu {
    pub fn new() -> Self {
        SittingMenu
    }
}

impl Default for SittingMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl State<Parlor> for SittingMenu {
    fn name(&self) -> &'static str {
        "story"
    }

    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        "Stub chapter 1.".to_string()
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        parlor_core::menu_sounds::leave_game(ctx);
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.pressed() == Some(Key::Escape) {
            ctx.pop();
        }
    }
}
