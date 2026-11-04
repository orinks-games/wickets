use hearken::menu::{Menu, MenuItem};
use hearken::Ctx;
use parlor_core::Parlor;

pub fn waiting_line(_ctx: &mut Ctx<Parlor>) -> Option<String> {
    None
}

pub fn lobby_row<S: Menu<Game = Parlor>>(
    _ctx: &mut Ctx<Parlor>,
) -> Option<MenuItem<S, Parlor>> {
    None
}
