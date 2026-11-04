//! The daily, weekly and monthly challenges: their streets, the shared
//! menu of the three, the briefing read before one, and the stand-in
//! screen the launcher opens one from.

use hearken::text::sentence_start;
use hearken::{Ctx, State};
use parlor_core::menus::{self, ChallengeGame};
use parlor_core::{Challenge, Pages, Parlor, Period};

use crate::dance::DanceScreen;
use crate::rules::Street;
use crate::{sound, streets, ID};

/// The challenge menu's `State::name`.
pub const CHALLENGES_NAME: &str = "dragon_dance_challenges";

/// The challenge's name.
pub fn name(period: Period) -> &'static str {
    match period {
        Period::Daily => "Today's street",
        Period::Weekly => "The long parade",
        Period::Monthly => "The grand festival",
    }
}

/// The street a challenge dances.
pub fn street(period: Period) -> Street {
    match period {
        Period::Daily => streets::DAILY,
        Period::Weekly => streets::WEEKLY,
        Period::Monthly => streets::MONTHLY,
    }
}

/// A few words on the challenge.
pub fn tag(period: Period) -> &'static str {
    match period {
        Period::Daily => "4 lanes, 24 bars",
        Period::Weekly => "4 lanes, 48 bars, a gong every other bar",
        Period::Monthly => "5 lanes, quick drums, firecrackers close behind",
    }
}

/// The rule, in a sentence or two.
pub fn rule(period: Period) -> String {
    let s = street(period);
    format!(
        "One street: {}. About {} seconds, at {} beats a minute.",
        s.describe(),
        s.seconds(),
        s.bpm
    )
}

/// The game's side of the shared challenges menu.
pub struct Dancers;

/// The three challenges, each with what it is and its record, then Back.
pub type ChallengesMenu = menus::ChallengesMenu<Dancers>;

impl ChallengeGame for Dancers {
    const ID: &'static str = ID;
    const NAME: &'static str = "Dragon Dance";
    const MENU_NAME: &'static str = CHALLENGES_NAME;

    fn name(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        name(period).to_string()
    }

    fn tag(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        tag(period).to_string()
    }

    fn help(_ctx: &mut Ctx<Parlor>, period: Period) -> String {
        rule(period)
    }

    fn open(ctx: &mut Ctx<Parlor>, period: Period) {
        let pages = briefing(ctx, period);
        ctx.push(pages);
    }

    fn music(ctx: &mut Ctx<Parlor>) {
        sound::title_music(ctx);
    }

    fn intro_help() -> String {
        "Up and Down choose a challenge, Enter reads its briefing, Escape goes back. Every player gets the same streets: a new one each day, each Monday and each month.".to_string()
    }
}

/// The pages read before a challenge, which then start it.
pub fn briefing(ctx: &mut Ctx<Parlor>, period: Period) -> Pages {
    let ch = Challenge::current(ctx, ID, period);
    let pages = vec![format!("{}: {}", name(period), rule(period))];
    let mut ch = Some(ch);
    Pages::briefing(pages).then(move |ctx| {
        if let Some(ch) = ch.take() {
            ctx.replace(DanceScreen::challenge(ch));
        }
    })
}

/// Stands in for the challenges menu when the launcher opens a challenge:
/// it opens the briefing, and once the challenge is over (or left from the
/// pause menu) it steps aside, back to the launcher.
pub struct ChallengeHost {
    period: Period,
    opened: bool,
}

impl ChallengeHost {
    pub fn new(period: Period) -> ChallengeHost {
        ChallengeHost {
            period,
            opened: false,
        }
    }
}

impl State<Parlor> for ChallengeHost {
    /// Named as the challenges menu is, so leaving a challenge lands here
    /// and steps aside to the launcher.
    fn name(&self) -> &'static str {
        CHALLENGES_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        if !self.opened {
            self.opened = true;
            sound::title_music(ctx);
            let pages = briefing(ctx, self.period);
            ctx.push(pages);
        }
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.audio.play_music(None, false);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop();
    }

    fn resume_quiet(&mut self, ctx: &mut Ctx<Parlor>) {
        ctx.pop();
    }

    fn resume_with(&mut self, ctx: &mut Ctx<Parlor>, _result: &str) {
        ctx.pop();
    }

    fn help(&self, _ctx: &Ctx<Parlor>) -> String {
        format!(
            "Opening the Dragon Dance challenge: {}.",
            sentence_start(name(self.period))
        )
    }
}
