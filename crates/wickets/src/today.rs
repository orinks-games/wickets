//! Happening today: everything on today in one list, reachable from the
//! lobby. The launch line names one event and counts the rest; this list
//! keeps all of them a row away, with the story's Happening and the challenges, and Enter on
//! a row goes to it.

use hearken::menu::{Menu, MenuCore, MenuItem};
use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::calendar::days_phrase;
use parlor_core::{challenges, Date, GameInfo, Parlor, Period};

use crate::lobby::{
    after_comma, back_row, happening, happenings, open_game, period_help, period_label,
};
use crate::{games, happening as story_happening, music, ChallengeList};

/// The list's name, as the lobby row, the launch line and help say it.
pub const LABEL: &str = "Happening today";

/// How far ahead a quiet day looks for the next event.
const LOOK_AHEAD: i64 = 60;

/// The lobby's row: "Happening today, 3 events".
pub fn lobby_row<S: Menu<Game = Parlor>>(ctx: &mut Ctx<Parlor>) -> MenuItem<S, Parlor> {
    let label = match happenings(ctx).len() {
        0 => LABEL.to_string(),
        1 => format!("{LABEL}, 1 event"),
        n => format!("{LABEL}, {n} events"),
    };
    MenuItem::new(label, |_s: &mut S, ctx: &mut Ctx<Parlor>| {
        ctx.push(TodayList::new())
    })
    .help("Today's game events, the story's Happening and the challenges, each one Enter away.")
}

/// The next game event after a quiet `today`, with how many days off. A
/// holiday special counts only once the lists show it, in season or kept,
/// so a quiet day never names one before its season. An event the player
/// has already finished is left out, and so is a category or game left
/// out in Game preferences.
pub fn next_event(ctx: &mut Ctx<Parlor>, today: Date) -> Option<(GameInfo, String, i64)> {
    let prefs = parlor_core::preferences::load(ctx);
    let specials = crate::specials_listed(ctx);
    (1..=LOOK_AHEAD).find_map(|ahead| {
        let day = Date::from_days(today.days() + ahead);
        let games = games().into_iter().chain(specials.iter().copied());
        games.filter(|g| prefs.shows(g)).find_map(|g| {
            let h = g.event_on(ctx, day)?;
            Some((g, h.title, ahead))
        })
    })
}

/// "No events today. Next: Stargazer, the Hunter's Moon, in 3 days."
fn quiet_line(ctx: &mut Ctx<Parlor>) -> String {
    let today = challenges::today(ctx);
    match next_event(ctx, today) {
        Some((g, title, days)) => format!(
            "No events today. Next: {}, {}, {}.",
            g.name,
            after_comma(&title),
            days_phrase(days)
        ),
        None => "No events today.".to_string(),
    }
}

/// Everything on today: game events, the story's Happening, then the
/// daily, weekly and monthly challenges.
pub struct TodayList {
    menu: MenuCore<Self, Parlor>,
}

impl TodayList {
    pub fn new() -> Self {
        TodayList {
            menu: MenuCore::new(LABEL).with_intro_help(
                "Up and Down choose what's on, Enter goes to it, Escape goes back to the lobby.",
            ),
        }
    }
}

impl Default for TodayList {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu for TodayList {
    type Game = Parlor;

    fn name(&self) -> &'static str {
        "today"
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        parlor_core::menu_sounds::use_shared(ctx);
        music::lobby(ctx);
        hearken::menu::menu_enter(self, ctx);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        parlor_core::menu_sounds::use_shared(ctx);
        music::lobby(ctx);
        self.refresh(ctx, true);
        self.announce(ctx);
    }

    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        parlor_core::menu_sounds::use_shared(ctx);
        music::lobby(ctx);
        self.refresh(ctx, true);
    }

    fn resume_with(&mut self, ctx: &mut Ctx<Parlor>, result: &str) {
        parlor_core::menu_sounds::use_shared(ctx);
        music::lobby(ctx);
        self.refresh(ctx, true);
        ctx.say(hearken::text::end_sentence(result));
        let current = self.current_text(ctx);
        ctx.say_queued(current);
    }

    fn menu(&self) -> &MenuCore<Self, Parlor> {
        &self.menu
    }

    fn menu_mut(&mut self) -> &mut MenuCore<Self, Parlor> {
        &mut self.menu
    }

    fn build_items(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<MenuItem<Self, Parlor>> {
        let mut items = Vec::new();
        let events = happenings(ctx);
        if events.is_empty() {
            let line = quiet_line(ctx);
            let again = line.clone();
            items.push(
                MenuItem::new(
                    "No events today",
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| ctx.say(again.clone()),
                )
                .help(line),
            );
        }
        for (g, h) in events {
            items.push(
                MenuItem::new(
                    format!("{}, {}", g.name, after_comma(&h.title)),
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        // Checked again, in case midnight passed.
                        match happening(ctx, &g) {
                            Some(h) => {
                                ctx.game.title_focus = Some(h.focus());
                                open_game(ctx, g)
                            }
                            None => ctx.say(format!("{} has no event today any more.", g.name)),
                        }
                    },
                )
                .help(h.line),
            );
        }
        if let Some(h) = crate::story::happening_now(ctx) {
            let what = story_happening::what(&h);
            let waits = crate::story::errand_waits(ctx);
            let help = if h.over() {
                "Over for this sitting. A new Happening comes with the next one.".to_string()
            } else if waits {
                "Its game was skipped, so this errand waits for another day.".to_string()
            } else {
                h.text.clone()
            };
            let state = if waits {
                ": waits"
            } else {
                story_happening::state(&h)
            };
            items.push(
                MenuItem::new(
                    format!("Story, {what}{state}"),
                    |_s: &mut Self, ctx: &mut Ctx<Parlor>| crate::story::open_at_happening(ctx),
                )
                .help(help),
            );
        }
        for period in Period::ALL {
            items.push(
                MenuItem::new(
                    sentence_start(&period_label(ctx, period)),
                    move |_s: &mut Self, ctx: &mut Ctx<Parlor>| {
                        ctx.push(ChallengeList::new(period))
                    },
                )
                .help(period_help(period)),
            );
        }
        items.push(back_row("Return to the lobby."));
        items
    }
}
