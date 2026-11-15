//! Story, help and debriefs, one page at a time.

use hearken::{Bus, Ctx, InputEvent, Key, State, UiSound};

use crate::{Parlor, ParlorAction, Records};

/// The shared cue played as each page begins. A game may ship its own at
/// `sounds/<id>/menu/page.ogg` (see [`crate::menu_sounds`]).
pub const PAGE_SOUND: &str = "parlor/page";

type Done = Box<dyn FnMut(&mut Ctx<Parlor>)>;

/// Longer text as a run of short pages: a soft cue, one page spoken, Enter
/// for the next (said after the first page only, and never again in a
/// game once the player has pressed it there; a briefing's last page still
/// says Enter starts), the repeat key (R) to hear a page again, Up and Down to
/// step back and forth through the pages (saying the page again at either
/// end), Escape to skip to the end. Enter always goes on from the furthest
/// page heard, so after going back over earlier pages it never steps
/// through them again. Nothing else runs underneath while it is open, so a game's
/// clock stops while the player reads.
pub struct Pages {
    pages: Vec<String>,
    at: usize,
    /// The furthest page heard: Enter goes on from here, wherever Up and
    /// Down have left `at`.
    read: usize,
    done: Option<Done>,
    /// Read before play: Escape backs out instead of skipping to the end.
    briefing: bool,
    /// Music that starts as a page is read, by page: story mode plays a
    /// scene's own piece from its first page.
    music: Vec<Option<&'static str>>,
    /// The first page waits for current speech instead of cutting it off.
    queued: bool,
    /// The Enter hint has been said on these pages.
    hinted: bool,
    /// Opened over play: the game's sound and event speech wait while the
    /// pages are open, as on the pause screen.
    hold: bool,
}

/// Explanation the player asked for during play, such as F1: said as it
/// is when it fits on one short page, else read as pages, visual-novel
/// style, with the game held still underneath until they close.
pub fn explain(ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
    let pages = crate::help::pages_of(&text.into());
    if pages.len() <= 1 {
        ctx.say(pages.concat());
    } else {
        ctx.push(Pages::new(pages).holding());
    }
}

/// Where a game keeps that its player knows Enter turns the page: under
/// the open game's id, or the launcher's own key in the lobby and story.
fn learned_key(ctx: &Ctx<Parlor>) -> String {
    match ctx.game.open.map(|g| g.id) {
        Some(id) => format!("{id}/pages_learned"),
        None => "parlor/pages_learned".to_string(),
    }
}

fn knows_enter(ctx: &mut Ctx<Parlor>) -> bool {
    let key = learned_key(ctx);
    Records::load(ctx, &key)
}

/// Saves that the player turned a page with Enter in this game, so its
/// pages stop saying how.
fn learn_enter(ctx: &mut Ctx<Parlor>) {
    if !knows_enter(ctx) {
        let key = learned_key(ctx);
        Records::store(ctx, &key, &true);
    }
}

impl Pages {
    /// Pages that close back to the screen beneath when read.
    pub fn new<S: Into<String>>(pages: impl IntoIterator<Item = S>) -> Pages {
        Pages {
            pages: pages.into_iter().map(Into::into).collect(),
            at: 0,
            read: 0,
            done: None,
            briefing: false,
            music: Vec::new(),
            queued: false,
            hinted: false,
            hold: false,
        }
    }

    /// Pages opened over play: the game's sound and event speech pause
    /// while they are open and go on when they close.
    pub fn holding(mut self) -> Pages {
        self.hold = true;
        self
    }

    /// Opens on page `at`, with the earlier ones a press of Up away: for a
    /// game that keeps its tutorial messages so far, so the player can go
    /// back over them.
    pub fn open_at(mut self, at: usize) -> Pages {
        self.at = at.min(self.pages.len().saturating_sub(1));
        self.read = self.at;
        self
    }

    /// Starts `music[i]`, looping, as page `i` is read, unless it is
    /// already playing. `None` leaves the music as it is.
    pub fn with_music(mut self, music: Vec<Option<&'static str>>) -> Pages {
        self.music = music;
        self
    }

    /// Pages read before play, such as a challenge briefing or a story
    /// intro. Enter on the last page runs the `then` callback, which starts
    /// play; Escape closes the pages without running it, back to the menu
    /// that opened them.
    pub fn briefing<S: Into<String>>(pages: impl IntoIterator<Item = S>) -> Pages {
        Pages {
            briefing: true,
            ..Pages::new(pages)
        }
    }

    /// What to do once the last page is read, or the pages are skipped,
    /// in place of closing: for example `ctx.replace(Run::new())`.
    pub fn then(mut self, done: impl FnMut(&mut Ctx<Parlor>) + 'static) -> Pages {
        self.done = Some(Box::new(done));
        self
    }

    /// The first page is queued behind what is being said, such as a
    /// tutorial's closing line, instead of interrupting it.
    pub fn after_speech(mut self) -> Pages {
        self.queued = true;
        self
    }

    fn speak(&mut self, ctx: &mut Ctx<Parlor>) {
        self.speak_with(ctx, false);
    }

    fn speak_with(&mut self, ctx: &mut Ctx<Parlor>, queued: bool) {
        if let Some(Some(piece)) = self.music.get(self.at) {
            if ctx.audio.music() != Some(*piece) {
                ctx.audio.play_music(Some(*piece), true);
            }
        }
        let cue = crate::menu_sounds::page(ctx);
        ctx.audio.play(cue, Bus::UI);
        let n = self.pages.len();
        let page = &self.pages[self.at];
        let last = self.at + 1 == n;
        // A briefing always says Enter starts, since its last page is a
        // choice between playing and going back. Turning a page is said
        // once, until the player has done it in this game.
        let hint = !self.hinted && !knows_enter(ctx);
        self.hinted = true;
        let line = if last && self.briefing {
            format!("{page} Enter to start.")
        } else if !hint {
            page.clone()
        } else if !last {
            format!("{page} Enter for more.")
        } else {
            format!("{page} Enter to continue.")
        };
        if queued {
            ctx.say_queued(line);
        } else {
            ctx.say(line);
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        match self.done.take() {
            Some(mut done) => done(ctx),
            None => ctx.pop(),
        }
    }
}

impl State<Parlor> for Pages {
    fn name(&self) -> &'static str {
        "pages"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.hold {
            ctx.audio.pause_all();
            ctx.pause_event_speech();
        }
        if self.pages.is_empty() {
            self.finish(ctx);
        } else {
            self.speak_with(ctx, self.queued);
        }
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        self.speak(ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.hold {
            ctx.audio.resume_all();
            ctx.resume_event_speech();
        }
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if event.is_repeat() {
            return;
        }
        let action = ctx.action_for(event);
        if action == Some(ParlorAction::Repeat) {
            self.speak(ctx);
            return;
        }
        // Up and Down browse, the way a player reads any list. At either
        // end they say the page again rather than leave it.
        let key = event.pressed();
        if action == Some(ParlorAction::Up) || key == Some(Key::Up) {
            self.at = self.at.saturating_sub(1);
            self.speak(ctx);
            return;
        }
        if action == Some(ParlorAction::Down) || key == Some(Key::Down) {
            self.at = (self.at + 1).min(self.pages.len().saturating_sub(1));
            self.read = self.read.max(self.at);
            self.speak(ctx);
            return;
        }
        match key {
            Some(Key::Return | Key::KpEnter | Key::Space) => {
                learn_enter(ctx);
                // Enter goes on: to the first page not yet heard, or out
                // once all have been, never back through pages re-read
                // with Up.
                if self.read + 1 < self.pages.len() {
                    self.read += 1;
                    self.at = self.read;
                    self.speak(ctx);
                } else {
                    self.finish(ctx);
                }
            }
            Some(Key::Escape) if self.briefing => {
                ctx.play_ui(UiSound::Back);
                ctx.pop();
            }
            Some(Key::Escape) => self.finish(ctx),
            _ => {}
        }
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        self.pages.get(self.at).cloned().into_iter().collect()
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let escape = if self.briefing {
            "Escape goes back without starting"
        } else {
            "Escape skips to the end"
        };
        let repeat = ctx.control_name(ParlorAction::Repeat);
        // One page has no pages to page through.
        if self.pages.len() == 1 {
            return format!("Enter goes on, and {repeat} repeats this page. {escape}.");
        }
        format!(
            "Page {} of {}. Enter goes on. Up and Down go back and forward a page, and {repeat} repeats this one. {escape}.",
            self.at + 1,
            self.pages.len(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::menus::testing::Base;
    use crate::OpenGame;
    use hearken::Headless;

    fn open(pages: impl FnOnce(Rc<Cell<bool>>) -> Pages) -> (Headless<Parlor>, Rc<Cell<bool>>) {
        let ran = Rc::new(Cell::new(false));
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.push(pages(Rc::clone(&ran)));
        h.frame(0.0);
        (h, ran)
    }

    fn flag(ran: Rc<Cell<bool>>) -> impl FnMut(&mut Ctx<Parlor>) {
        move |ctx: &mut Ctx<Parlor>| {
            ran.set(true);
            ctx.pop();
        }
    }

    #[test]
    fn explain_says_a_short_line_and_pages_a_long_one_over_the_game() {
        let mut h = Headless::new(Default::default(), Base);
        h.frame(0.0);
        h.clear();
        explain(&mut h.app.ctx, "Short help.");
        h.frame(0.0);
        assert_eq!(h.spoken(), ["Short help."]);
        assert_eq!(h.app.top_name(), "base");
        let sentence = "The cat in the pantry hunts you by the shortest way it can find.";
        explain(&mut h.app.ctx, [sentence; 8].join(" "));
        h.frame(0.0);
        assert_eq!(h.app.top_name(), "pages");
        for _ in 0..4 {
            h.press(Key::Return);
        }
        assert_eq!(h.app.top_name(), "base");
    }

    #[test]
    fn escape_on_a_briefing_goes_back_without_starting() {
        let (mut h, ran) = open(|r| Pages::briefing(["One.", "Two."]).then(flag(r)));
        h.press(Key::Escape);
        assert!(!ran.get());
        assert_eq!(h.app.top_name(), "base");
    }

    #[test]
    fn enter_on_a_briefing_last_page_starts() {
        let (mut h, ran) = open(|r| Pages::briefing(["One.", "Two."]).then(flag(r)));
        assert_eq!(h.spoken(), ["One. Enter for more."]);
        h.press(Key::Return);
        assert_eq!(
            h.spoken().last().map(String::as_str),
            Some("Two. Enter to start.")
        );
        h.press(Key::Return);
        assert!(ran.get());
    }

    #[test]
    fn a_single_briefing_page_says_enter_starts() {
        let (h, _) = open(|r| Pages::briefing(["Only."]).then(flag(r)));
        assert_eq!(h.spoken(), ["Only. Enter to start."]);
    }

    #[test]
    fn help_on_a_single_page_does_not_offer_paging() {
        let (mut h, _) = open(|r| Pages::briefing(["Only."]).then(flag(r)));
        h.clear();
        h.press(Key::F1);
        let said = h.spoken().join(" ");
        assert!(said.starts_with("Enter goes on, and "), "{said}");
        assert!(!said.contains("Page 1 of 1"), "{said}");
        assert!(!said.contains("Up and Down"), "{said}");
        let (mut h, _) = open(|r| Pages::briefing(["One.", "Two."]).then(flag(r)));
        h.clear();
        h.press(Key::F1);
        assert!(spoke_page(&h), "{:?}", h.spoken());
    }

    fn spoke_page(h: &Headless<Parlor>) -> bool {
        h.spoken().iter().any(|l| l.starts_with("Page 1 of 2."))
    }

    #[test]
    fn up_and_down_browse_and_say_the_page_again_at_the_ends() {
        let (mut h, ran) = open(|r| Pages::new(["One.", "Two."]).then(flag(r)));
        h.clear();
        h.press(Key::Up);
        assert_eq!(h.spoken(), ["One."]);
        h.clear();
        h.press(Key::Down);
        assert_eq!(h.spoken(), ["Two."]);
        h.clear();
        h.press(Key::Down);
        assert_eq!(h.spoken(), ["Two."]);
        assert!(!ran.get(), "Down never closes the pages");
        h.clear();
        h.press(Key::Up);
        assert_eq!(h.spoken(), ["One."]);
    }

    #[test]
    fn open_at_starts_on_that_page_with_the_rest_behind() {
        let (mut h, ran) = open(|r| Pages::new(["One.", "Two."]).open_at(1).then(flag(r)));
        assert_eq!(h.spoken(), ["Two. Enter to continue."]);
        h.clear();
        h.press(Key::Up);
        assert_eq!(h.spoken(), ["One."]);
        h.press(Key::Return);
        assert!(ran.get());
    }

    #[test]
    fn the_hint_comes_back_until_enter_is_pressed_then_stops_for_that_game() {
        let (mut h, _) = open(|_| Pages::new(["One.", "Two."]));
        h.app.ctx.game.open = Some(OpenGame::new("lamplighter", "Lamplighter", |_| Vec::new()));
        // Left with Escape: the player has not pressed Enter yet.
        h.press(Key::Escape);
        h.clear();
        h.app.ctx.push(Pages::new(["One.", "Two."]));
        h.frame(0.0);
        assert_eq!(h.spoken(), ["One. Enter for more."]);
        h.press(Key::Return);
        h.press(Key::Return);
        h.clear();
        h.app.ctx.push(Pages::new(["Three."]));
        h.frame(0.0);
        assert_eq!(h.spoken(), ["Three."], "learned in this game");
        h.press(Key::Return);
        h.clear();
        h.app.ctx.game.open = Some(OpenGame::new("belfry", "Belfry", |_| Vec::new()));
        h.app.ctx.push(Pages::new(["Four."]));
        h.frame(0.0);
        assert_eq!(
            h.spoken(),
            ["Four. Enter to continue."],
            "another game still says it"
        );
    }

    #[test]
    fn enter_after_going_back_continues_instead_of_paging_again() {
        let (mut h, ran) = open(|r| {
            Pages::new(["One.", "Two.", "Three."])
                .open_at(2)
                .then(flag(r))
        });
        h.press(Key::Up);
        h.press(Key::Up);
        h.press(Key::Return);
        assert!(ran.get(), "every page was heard, so Enter closes them");
    }

    #[test]
    fn enter_after_going_back_reads_the_next_unheard_page() {
        let (mut h, ran) = open(|r| Pages::new(["One.", "Two.", "Three."]).then(flag(r)));
        h.press(Key::Return);
        h.press(Key::Up);
        h.clear();
        h.press(Key::Return);
        assert_eq!(h.spoken(), ["Three."]);
        assert!(!ran.get());
        h.press(Key::Return);
        assert!(ran.get());
    }

    #[test]
    fn escape_on_plain_pages_still_runs_then() {
        let (mut h, ran) = open(|r| Pages::new(["One.", "Two."]).then(flag(r)));
        h.press(Key::Escape);
        assert!(ran.get());
        assert_eq!(h.app.top_name(), "base");
    }
}
