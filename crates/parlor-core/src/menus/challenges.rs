//! Every game's challenges menu: the daily, weekly and monthly challenge,
//! then Back. The rows read the same way in every game:
//! "Daily, Tonight's job: one safe, with quiet tumblers. Not played yet."

use std::marker::PhantomData;

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::{end_sentence, sentence_start};
use hearken::{Ctx, UiSound};

use super::labels;
use crate::{Challenge, Challenges, Parlor, Period};

/// What a game gives the shared challenges menu.
pub trait ChallengeGame: 'static {
    /// The game's id, as its challenges are saved under.
    const ID: &'static str;
    /// The game's name, for the Back row's help.
    const NAME: &'static str;
    /// The menu's `State::name`: the title's name plus `_challenges`, such
    /// as `"safecracker_challenges"`. Results and pause return here.
    const MENU_NAME: &'static str;

    /// The challenge's own name: "Tonight's job".
    fn name(ctx: &mut Ctx<Parlor>, period: Period) -> String;
    /// A few words on what it is this time: "one safe, with quiet tumblers".
    fn tag(ctx: &mut Ctx<Parlor>, period: Period) -> String;
    /// F1 on its row: the rule in a sentence or two.
    fn help(ctx: &mut Ctx<Parlor>, period: Period) -> String;
    /// Enter on its row: usually pushes a `Pages::briefing` that starts it.
    fn open(ctx: &mut Ctx<Parlor>, period: Period);

    /// Starts the menu's music, on entering and on every return.
    fn music(_ctx: &mut Ctx<Parlor>) {}
    /// Called when the menu closes, such as to bring the title's music back.
    fn leave(_ctx: &mut Ctx<Parlor>) {}
    /// F1 on the menu itself.
    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same challenges.".to_string()
    }
}

/// "not played yet", "done, best 1200, a 3 day streak": the record part of
/// a challenge row, without the period.
pub fn record_words(ctx: &mut Ctx<Parlor>, challenge: &Challenge) -> String {
    let summary = Challenges::summary(ctx, challenge);
    let prefix = format!("{}, ", challenge.title());
    summary
        .strip_prefix(&prefix)
        .map(str::to_string)
        .unwrap_or(summary)
}

/// A challenge row, as every game's menu reads it:
/// "Daily, Tonight's job: one safe, with quiet tumblers. Not played yet".
pub fn row_label(
    ctx: &mut Ctx<Parlor>,
    id: &'static str,
    period: Period,
    name: &str,
    tag: &str,
) -> String {
    let challenge = Challenge::current(ctx, id, period);
    let record = record_words(ctx, &challenge);
    format!(
        "{}, {name}: {} {}",
        period.label(),
        end_sentence(tag),
        sentence_start(&record)
    )
}

/// The shared challenges menu for game `G`.
pub struct ChallengesMenu<G: ChallengeGame> {
    menu: MenuCore<Self, Parlor>,
    game: PhantomData<fn() -> G>,
}

impl<G: ChallengeGame> ChallengesMenu<G> {
    pub fn new() -> Self {
        ChallengesMenu {
            menu: MenuCore::new(labels::CHALLENGES).with_intro_help(G::intro_help()),
            game: PhantomData,
        }
    }
}

impl<G: ChallengeGame> Default for ChallengesMenu<G> {
    fn default() -> Self {
        Self::new()
    }
}

impl<G: ChallengeGame> Menu for ChallengesMenu<G> {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        G::MENU_NAME
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        G::music(ctx);
        hearken::menu::menu_enter(self, ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        G::music(ctx);
        self.refresh(ctx, true);
        self.announce(ctx);
    }

    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        G::music(ctx);
        self.refresh(ctx, true);
    }

    fn resume_with(&mut self, ctx: &mut Ctx<Parlor>, result: &str) {
        G::music(ctx);
        self.refresh(ctx, true);
        ctx.say(end_sentence(result));
        let current = self.current_text(ctx);
        ctx.say_queued(current);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        G::leave(ctx);
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items: Vec<MenuItem<Self, Parlor>> = Period::ALL
            .into_iter()
            .map(|period| {
                let name = G::name(ctx, period);
                let tag = G::tag(ctx, period);
                let label = row_label(ctx, G::ID, period, &name, &tag);
                let resets = Challenge::current(ctx, G::ID, period).resets();
                let help = format!("{} {resets}", end_sentence(&G::help(ctx, period)));
                MenuItem::new(label, move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                    G::open(ctx, period)
                })
                .help(help)
            })
            .collect();
        items.push(
            MenuItem::new(labels::BACK, |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                ctx.play_ui(UiSound::Back);
                ctx.pop()
            })
            .help(format!("Return to the {} title.", G::NAME)),
        );
        items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::testing::{rows, Base};
    use crate::{Date, Pages};
    use hearken::{Headless, Key};

    struct Jobs;

    impl ChallengeGame for Jobs {
        const ID: &'static str = "jobs";
        const NAME: &'static str = "Jobs";
        const MENU_NAME: &'static str = "jobs_challenges";

        fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
            match period {
                Period::Daily => "Tonight's job",
                Period::Weekly => "The vault",
                Period::Monthly => "The heist",
            }
            .to_string()
        }
        fn tag(_ctx: &mut Ctx<Parlor>, _period: Period) -> String {
            "one safe, with quiet tumblers".to_string()
        }
        fn help(_ctx: &mut Ctx<Parlor>, _period: Period) -> String {
            "Crack it".to_string()
        }
        fn open(ctx: &mut Ctx<Parlor>, _period: Period) {
            ctx.push(Pages::briefing(["The briefing."]).then(|ctx| ctx.say("Started.")));
        }
    }

    fn open() -> Headless<Parlor> {
        let mut h = Headless::new(Default::default(), Base);
        h.app.ctx.game.today = Some(Date::new(2026, 10, 16));
        h.app.ctx.push(ChallengesMenu::<Jobs>::new());
        h.frame(0.0);
        h
    }

    #[test]
    fn rows_lead_with_the_period_then_the_name_tag_and_record() {
        let mut h = open();
        assert_eq!(h.app.top_name(), "jobs_challenges");
        assert_eq!(
            rows(&h),
            [
                "Daily, Tonight's job: one safe, with quiet tumblers. Not played yet",
                "Weekly, The vault: one safe, with quiet tumblers. Not played yet",
                "Monthly, The heist: one safe, with quiet tumblers. Not played yet",
                "Back",
            ]
        );
        let daily = Challenge::current(&h.app.ctx, "jobs", Period::Daily);
        Challenges::submit(&mut h.app.ctx, &daily, 300);
        h.press(Key::Return);
        h.press(Key::Escape);
        assert_eq!(
            rows(&h)[0],
            "Daily, Tonight's job: one safe, with quiet tumblers. Done, best 300, a 1 day streak"
        );
        h.clear();
        h.press(Key::F1);
        assert_eq!(h.spoken(), ["Crack it. A new one tomorrow."]);
    }

    #[test]
    fn escape_and_back_both_return() {
        let mut h = open();
        h.press(Key::Escape);
        assert_eq!(h.app.top_name(), "base");
        let mut h = open();
        h.press(Key::End);
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "base");
    }

    #[test]
    fn briefing_escape_comes_back_without_starting() {
        let mut h = open();
        h.press(Key::Return);
        assert_eq!(h.app.top_name(), "pages");
        h.clear();
        h.press(Key::Escape);
        assert_eq!(h.app.top_name(), "jobs_challenges");
        assert!(!h.spoken().iter().any(|l| l == "Started."));
    }
}
