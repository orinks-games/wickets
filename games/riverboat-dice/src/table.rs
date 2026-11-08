//! The dice table: calling rolls, banking, hearing the other players, and
//! the end of the night.

use std::cell::Cell;

use hearken::text::sentence_start;
use hearken::{Ctx, InputEvent, Key, Sound, State};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{Challenge, Pages, Parlor, ParlorAction, Period, Records, ScoreResult};

use crate::challenge::{self, Job, CHALLENGES_NAME};
use crate::progress::{self, Progress, Saved};
use crate::rules::{
    advice, odds_words, run_seed, Advice, Cue, LandingEnd, Night, Prompt, Refusal, RollReport,
    Rules, Sfx, Table, Who, MOON_TABLE, SALOON, YOU,
};
use crate::{events, sound, HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const TABLE_NAME: &str = "riverboat_dice_table";

/// How many times the stay-or-bank prompt names the keys in a night.
const DECIDE_PROMPTS: u32 = 2;

pub struct TableScreen {
    pub table: Table,
    /// A challenge, night after night, and the challenge it counts for.
    job: Option<Job>,
    challenge: Option<Challenge>,
    /// The opponent Left and Right last named, by seat.
    cursor: usize,
    /// The first bank of a first game asks first.
    confirm_bank: bool,
    /// A bank waiting for its confirmation.
    confirming: bool,
    decide_told: u32,
    /// The results are on their way: the night is over.
    ended: bool,
    band: Option<Sound>,
    /// High water: the river's rush under the table, in spring. Set on
    /// entering, from the date, and never in a challenge.
    pub high_water: bool,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
    /// Seconds the dealer waits before going on, such as while a
    /// tutorial step is read.
    hold: Cell<f64>,
}

impl TableScreen {
    /// A saloon night.
    pub fn new() -> TableScreen {
        TableScreen::with_seed(run_seed())
    }

    pub fn with_seed(seed: u64) -> TableScreen {
        TableScreen::with_night(Night::new(SALOON, seed))
    }

    /// A night set up by the caller, such as with scripted dice.
    pub fn with_night(night: Night) -> TableScreen {
        TableScreen {
            status_key: Default::default(),
            hold: Cell::new(0.0),
            table: Table::new(night),
            job: None,
            challenge: None,
            cursor: 0,
            confirm_bank: false,
            confirming: false,
            decide_told: 0,
            ended: false,
            band: None,
            high_water: false,
        }
    }

    /// A Showboat Saturday night.
    pub fn moon() -> TableScreen {
        TableScreen::with_night(Night::new(MOON_TABLE, run_seed()))
    }

    /// A Showboat Saturday night with these dice.
    pub fn moon_with_seed(seed: u64) -> TableScreen {
        TableScreen::with_night(Night::new(MOON_TABLE, seed))
    }

    /// The tutorial's table: one landing, then the dealer stops.
    pub fn lesson(mut self) -> TableScreen {
        self.table.lesson = true;
        self
    }

    /// Makes the dealer wait before the next cue.
    pub fn hold(&self, secs: f64) {
        self.hold.set(secs);
    }

    pub fn is_moon(&self) -> bool {
        self.rules().moon
    }

    /// The challenge's nights, from its seed.
    pub fn challenge(ch: Challenge) -> TableScreen {
        let job = Job::new(&ch);
        let mut screen = TableScreen::with_night(job.night());
        screen.job = Some(job);
        screen.challenge = Some(ch);
        screen
    }

    pub fn is_challenge(&self) -> bool {
        self.job.is_some()
    }

    fn night(&self) -> &Night {
        &self.table.night
    }

    fn rules(&self) -> Rules {
        self.night().rules
    }

    fn opponents(&self) -> usize {
        self.night().seats.len() - 1
    }

    fn intro(&self) -> String {
        let n = self.night();
        let field = n.names(&(1..n.seats.len()).collect::<Vec<_>>());
        let snag = if n.rules.snag {
            " The snag rule is on."
        } else if n.rules.moon {
            " Doubles with no 1 double what the table holds."
        } else {
            ""
        };
        let what = match &self.job {
            Some(job) if job.nights() > 1 => format!(
                "{}. {}",
                challenge::name(job.period),
                challenge::next_words(job)
            ),
            Some(job) => format!("{}.", challenge::name(job.period)),
            None if n.rules.moon => format!("{}.", events::MOON_NAME),
            None => "Saloon.".to_string(),
        };
        format!(
            "{what} {} landings against {field}.{snag}",
            n.rules.landings
        )
    }

    fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let l = ctx.control_name(ParlorAction::Left);
        let r = ctx.control_name(ParlorAction::Right);
        let u = ctx.control_name(ParlorAction::Up);
        let d = ctx.control_name(ParlorAction::Down);
        let p = ctx.control_name(ParlorAction::Primary);
        let t = ctx.control_name(ParlorAction::Secondary);
        let s = ctx.control_name(ParlorAction::Status);
        let rep = ctx.control_name(ParlorAction::Repeat);
        let h = ctx.control_name(ParlorAction::Hint);
        format!(
            "{} stays in for a roll, {t} banks. {l} and {r} hear each player, {u} the odds, {d} the night totals. {s} gives the status, twice for more, {rep} the last roll, {h} a hint. F1 helps, Escape pauses.",
            sentence_start(&p),
            l = sentence_start(&l),
        )
    }

    fn first_roll_words(&self, ctx: &Ctx<Parlor>) -> String {
        format!(
            "{} calls the first roll.",
            sentence_start(&ctx.control_name(ParlorAction::Primary))
        )
    }

    /// Status, in a fixed order: the landing, the table, your night, your
    /// seat, then who is still in.
    /// The status key, in layers. First: the landing, the table and your
    /// seat. A second press: your night total, who is still in and the
    /// night's extras.
    pub fn status_layers(&self) -> Vec<String> {
        let n = self.night();
        if n.over {
            return vec![format!(
                "The night is over. Night totals: {}.",
                n.standings()
            )];
        }
        let you = if n.you().seated {
            "you are seated".to_string()
        } else {
            format!("you banked {}", n.you().banked.unwrap_or(0))
        };
        let seated = n.seated();
        let still = if seated.is_empty() {
            "nobody is seated".to_string()
        } else {
            format!("still in: {}", n.names(&seated))
        };
        let first = [
            format!("Landing {} of {}", n.landing, n.rules.landings),
            format!("the table holds {}", n.pot),
            you,
        ];
        let mut more = vec![format!("Your night {}", n.you().night), still];
        if self.high_water {
            more.push("high water on the river".to_string());
        }
        if let Some(job) = &self.job {
            if job.nights() > 1 {
                more.push(format!(
                    "night {} of {}, {} banked in earlier nights",
                    job.index + 1,
                    job.nights(),
                    job.banked
                ));
            }
        }
        vec![
            format!("{}.", first.join(", ")),
            format!("{}.", more.join(", ")),
        ]
    }

    /// Every layer of the status key at once.
    pub fn status(&self) -> String {
        self.status_layers().join(" ")
    }

    /// One opponent, as Left and Right read them.
    pub fn seat_words(&self, seat: usize) -> String {
        let n = self.night();
        let s = &n.seats[seat];
        let Who::Ai(o) = s.who else {
            return String::new();
        };
        let state = match (s.seated, s.banked) {
            (true, _) => "seated".to_string(),
            (false, Some(b)) if !n.over => format!("standing, banked {b}"),
            _ => "standing".to_string(),
        };
        format!(
            "{}, {}: {}, {state}. {}",
            o.title(),
            sound::side_words(seat, self.opponents()),
            s.night,
            o.rule()
        )
    }

    /// The last roll and what it did, for Repeat.
    pub fn last_roll_words(&self, ctx: &Ctx<Parlor>) -> String {
        let n = self.night();
        let Some(report) = &n.last else {
            return format!("No roll yet. {}", self.first_roll_words(ctx));
        };
        roll_story(n, report)
    }

    fn hint_words(&self, ctx: &Ctx<Parlor>) -> String {
        let mut text = self.advice_words(ctx);
        if self.high_water {
            text.push_str(" High water changes nothing at the table.");
        }
        text
    }

    fn advice_words(&self, ctx: &Ctx<Parlor>) -> String {
        let p = ctx.control_name(ParlorAction::Primary);
        let t = ctx.control_name(ParlorAction::Secondary);
        match advice(self.night()) {
            Advice::FirstRoll => format!("Roll: nothing at risk. {} rolls.", sentence_start(&p)),
            Advice::Stay(why) => format!("Stay in. {why} {} stays in.", sentence_start(&p)),
            Advice::Bank(why) => format!("Bank. {why} {} banks.", sentence_start(&t)),
            Advice::Standing => "You have banked. Listen to the others.".to_string(),
            Advice::Over => "The night is over.".to_string(),
        }
    }

    fn nope(&self, ctx: &mut Ctx<Parlor>, text: impl Into<String>) {
        sound::play(ctx, sound::NOPE);
        ctx.say(text.into());
    }

    fn play_cues(&mut self, ctx: &mut Ctx<Parlor>, cues: Vec<Cue>) {
        let opponents = self.opponents();
        for cue in cues {
            match cue {
                Cue::Sound(sfx) => {
                    sound::play_sfx(ctx, sfx);
                }
                Cue::Chair(seat) => {
                    sound::chair(ctx, seat, opponents);
                }
                Cue::Say(text) => ctx.say(text),
                Cue::Queue(text) => ctx.say_queued(text),
                Cue::Prompt(Prompt::Decide) => {
                    if self.decide_told < DECIDE_PROMPTS {
                        self.decide_told += 1;
                        let p = ctx.control_name(ParlorAction::Primary);
                        let t = ctx.control_name(ParlorAction::Secondary);
                        ctx.say_queued(format!(
                            "{} stays in, {t} banks {}.",
                            sentence_start(&p),
                            self.night().pot
                        ));
                    }
                }
                Cue::Prompt(Prompt::FirstRoll) => {
                    let text = self.first_roll_words(ctx);
                    ctx.say_queued(text);
                }
                Cue::NightOver | Cue::AutoRoll => {}
            }
        }
    }

    /// A key while the dealer talks: his words come at once.
    fn skip(&mut self, ctx: &mut Ctx<Parlor>) {
        let cues = self.table.skip();
        let spoken: Vec<Cue> = cues
            .into_iter()
            .map(|c| match c {
                Cue::Say(t) => Cue::Queue(t),
                other => other,
            })
            .collect();
        if spoken.iter().any(|c| matches!(c, Cue::Queue(_))) {
            ctx.stop_speech();
            self.play_cues(ctx, spoken);
        } else {
            self.play_cues(ctx, spoken);
            self.nope(ctx, "The dealer is rolling.");
        }
        self.check_end(ctx);
    }

    fn roll(&mut self, ctx: &mut Ctx<Parlor>) {
        self.confirming = false;
        match self.table.call_roll() {
            Ok(_) => {
                let due = self.table.update(0.0);
                self.play_cues(ctx, due);
            }
            Err(Refusal::Standing) => self.nope(
                ctx,
                format!(
                    "You already banked {}.",
                    self.night().you().banked.unwrap_or(0)
                ),
            ),
            Err(_) => self.nope(ctx, "The night is over."),
        }
    }

    fn bank(&mut self, ctx: &mut Ctx<Parlor>) {
        let n = self.night();
        let pot = n.pot;
        let can_bank = n.you().seated && pot > 0 && !n.over;
        if self.confirm_bank && !self.confirming && can_bank {
            self.confirming = true;
            let p = ctx.control_name(ParlorAction::Primary);
            let t = ctx.control_name(ParlorAction::Secondary);
            ctx.say(format!(
                "Bank {} and stand for the rest of this landing? {} again banks, {p} stays in.",
                pot,
                sentence_start(&t)
            ));
            return;
        }
        self.confirming = false;
        match self.table.bank() {
            Ok(_) => {
                self.confirm_bank = false;
                let due = self.table.update(0.0);
                self.play_cues(ctx, due);
            }
            Err(Refusal::Standing) => self.nope(
                ctx,
                format!(
                    "You banked {} this landing already.",
                    self.night().you().banked.unwrap_or(0)
                ),
            ),
            Err(Refusal::NothingToBank) => {
                let text = format!("Nothing to bank yet. {}", self.first_roll_words(ctx));
                self.nope(ctx, text);
            }
            Err(Refusal::Over) => self.nope(ctx, "The night is over."),
        }
    }

    fn look(&mut self, ctx: &mut Ctx<Parlor>, forward: bool) {
        let n = self.opponents();
        let at = if self.cursor == 0 {
            if forward {
                1
            } else {
                n
            }
        } else if forward {
            self.cursor % n + 1
        } else {
            (self.cursor + n - 2) % n + 1
        };
        self.cursor = at;
        let text = self.seat_words(at);
        ctx.say(text);
    }

    /// Once the night's last words are out, the results follow.
    fn check_end(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended || !self.table.finished() {
            return;
        }
        if self.job.is_some() {
            self.finish_job_night(ctx);
        } else {
            self.finish(ctx);
        }
    }

    fn stinger(&self, ctx: &mut Ctx<Parlor>) {
        if self.night().winners().contains(&YOU) {
            sound::play_sfx(ctx, Sfx::Cheer);
        }
    }

    fn finish(&mut self, ctx: &mut Ctx<Parlor>) {
        self.ended = true;
        self.stinger(ctx);
        let night = self.night().clone();
        if night.rules.moon {
            progress::note_stay(ctx, &night);
            let score = Records::record(ctx, ID, events::MOON_BOARD, night.you().night as u64);
            let pages = moon_pages(&night, score);
            let summary = result_summary(&night);
            let mut results = Some(after_moon(summary, pages.clone()));
            ctx.push(Pages::new(pages).then(move |ctx| {
                if let Some(results) = results.take() {
                    ctx.pop_to_quiet(TITLE_NAME);
                    ctx.push(results);
                }
            }));
            return;
        }
        let saved = progress::save_night(ctx, &night);
        let longest = Progress::load(ctx).longest_stay;
        let pages = result_pages(&night, &saved, longest);
        let summary = result_summary(&night);
        let mut results = Some(after_night(summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }

    /// A challenge night ended: on to the next, or the challenge is over.
    fn finish_job_night(&mut self, ctx: &mut Ctx<Parlor>) {
        let night = self.night().clone();
        progress::note_stay(ctx, &night);
        let Some(job) = self.job.as_mut() else {
            return;
        };
        if let Some(next) = job.advance(&night) {
            let done = outcome_words(&night);
            let intro = challenge::next_words(job);
            self.table = Table::new(next);
            self.cursor = 0;
            self.stinger_for(ctx, &night);
            ctx.say(done);
            ctx.say_queued(intro);
            let first = self.first_roll_words(ctx);
            ctx.say_queued(first);
            return;
        }
        self.ended = true;
        let (Some(job), Some(ch)) = (self.job.take(), self.challenge.take()) else {
            return;
        };
        self.stinger_for(ctx, &night);
        let pages = challenge::finish_job(ctx, &ch, &job, &night);
        let summary = format!("{} result: {} points", ch.title(), job.score());
        let mut results = Some(after_job(ch.period, summary, pages.clone()));
        ctx.replace(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.replace(results);
            }
        }));
    }

    fn stinger_for(&self, ctx: &mut Ctx<Parlor>, night: &Night) {
        if night.winners().contains(&YOU) {
            sound::play_sfx(ctx, Sfx::Cheer);
        }
    }

    fn stop_band(&mut self, ctx: &mut Ctx<Parlor>) {
        if let Some(band) = self.band.take() {
            ctx.audio.stop(band, 300);
        }
    }
}

impl Default for TableScreen {
    fn default() -> Self {
        TableScreen::new()
    }
}

/// A roll told in full, as Repeat reads it.
pub fn roll_story(n: &Night, report: &RollReport) -> String {
    let mut text = format!(
        "Last roll, landing {}: {}.",
        report.landing,
        report.roll.words()
    );
    if report.doubled {
        text.push_str(" Showboat doubles doubled the table.");
    }
    match report.end {
        Some(LandingEnd::One) => {
            text.push_str(" A one ended the landing.");
            let losers: Vec<usize> = report.lost.iter().map(|&(i, _)| i).collect();
            if !losers.is_empty() && report.pot > 0 {
                text.push_str(&format!(
                    " {} lost {}.",
                    sentence_start(&n.names(&losers)),
                    report.pot
                ));
            }
        }
        Some(LandingEnd::Snag) => {
            text.push_str(" The snag ended the landing.");
            let wiped: Vec<usize> = report.wiped.iter().map(|&(i, _)| i).collect();
            if !wiped.is_empty() {
                text.push_str(&format!(
                    " It wiped the night totals of {}.",
                    n.names(&wiped)
                ));
            }
        }
        _ => {
            text.push_str(&format!(" The table held {}.", report.pot));
            for &(i, amount) in &report.stood {
                text.push_str(&format!(
                    " {} stood with {amount}.",
                    sentence_start(n.seats[i].who.name())
                ));
            }
            if report.end == Some(LandingEnd::AllBanked) {
                text.push_str(" Everyone had banked.");
            }
        }
    }
    text
}

/// "first", "second"...
pub fn place_word(place: usize) -> &'static str {
    match place {
        1 => "first",
        2 => "second",
        3 => "third",
        _ => "fourth",
    }
}

/// How the night ended, in a sentence.
pub fn outcome_words(night: &Night) -> String {
    let winners = night.winners();
    let mine = night.you().night;
    if winners.is_empty() {
        return "Nobody banked tonight, so nobody wins.".to_string();
    }
    if winners == [YOU] {
        return format!("You win the night with {mine}.");
    }
    if winners.contains(&YOU) {
        let others: Vec<usize> = winners.iter().copied().filter(|&i| i != YOU).collect();
        return format!("A shared win with {}, {mine} each.", night.names(&others));
    }
    let top = winners[0];
    let lead = if winners.len() == 1 {
        format!(
            "{} wins with {}",
            sentence_start(night.seats[top].who.name()),
            night.seats[top].night
        )
    } else {
        format!(
            "{} share the win with {}",
            sentence_start(&night.names(&winners)),
            night.seats[top].night
        )
    };
    let place = night.place();
    let place = if place == night.seats.len() {
        "last"
    } else {
        place_word(place)
    };
    let level: Vec<usize> = (0..night.seats.len())
        .filter(|&i| i != YOU && night.seats[i].night == mine)
        .collect();
    if level.is_empty() {
        format!("{lead}. You came {place} with {mine}.")
    } else {
        format!(
            "{lead}. You came {place} with {mine}, level with {}.",
            night.names(&level)
        )
    }
}

/// The results menu's title.
pub fn result_summary(night: &Night) -> String {
    let words = outcome_words(night);
    words.trim_end_matches('.').to_string()
}

/// The pages after a saloon night.
pub fn result_pages(night: &Night, saved: &Saved, longest: u32) -> Vec<String> {
    let mine = night.you().night;
    let mut first = outcome_words(night);
    first.push_str(&match saved.score {
        ScoreResult::First => format!(" {mine} is your first saloon night."),
        ScoreResult::NewBest { previous } => {
            format!(" {mine} is a new best night, beating {previous}.")
        }
        ScoreResult::Short { best } => format!(" Your best night is {best}."),
    });
    let mut second = format!("Final totals: {}.", night.standings());
    if saved.wins > 0 {
        second.push_str(&format!(" Nights won so far: {}.", saved.wins));
    }
    let rolls = |n: u32| {
        if n == 1 {
            "1 roll".to_string()
        } else {
            format!("{n} rolls")
        }
    };
    if saved.longest_record {
        second.push_str(&format!(
            " A new longest stay: {} in one landing.",
            rolls(longest)
        ));
    } else {
        second.push_str(&format!(" Longest stay: {}.", rolls(longest)));
    }
    vec![first, second]
}

/// The results menu after a saloon night.
pub fn after_night(summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        labels::PLAY_AGAIN,
        "A new saloon night, with new dice.",
        |ctx| ctx.replace(TableScreen::new()),
    )
}

/// The pages after a Showboat Saturday night.
pub fn moon_pages(night: &Night, score: ScoreResult) -> Vec<String> {
    let mine = night.you().night;
    let mut first = outcome_words(night);
    first.push_str(&match score {
        ScoreResult::First => format!(" {mine} is your first Showboat Saturday."),
        ScoreResult::NewBest { previous } => {
            format!(" {mine} is a new showboat best, beating {previous}.")
        }
        ScoreResult::Short { best } => format!(" Your showboat best is {best}."),
    });
    vec![first, format!("Final totals: {}.", night.standings())]
}

/// The results menu after a Showboat Saturday night. Once it is over,
/// Play again says the table has ended and goes back to the title.
pub fn after_moon(summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Title(TITLE_NAME)).again(
        labels::PLAY_AGAIN,
        "Another Showboat Saturday night, with new dice.",
        |ctx| {
            if events::moon_open(parlor_core::challenges::today(ctx)) {
                ctx.replace(TableScreen::moon())
            } else {
                ctx.pop_to_with(TITLE_NAME, events::MOON_ENDED)
            }
        },
    )
}

/// The results menu after a challenge.
pub fn after_job(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same dice, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            ctx.replace(TableScreen::challenge(ch))
        },
    )
}

impl State<Parlor> for TableScreen {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        TABLE_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        let today = parlor_core::challenges::today(ctx);
        self.high_water = !self.is_challenge() && events::high_water(today);
        self.band = Some(sound::saloon(ctx, self.high_water));
        self.confirm_bank = !self.is_challenge() && Progress::load(ctx).nights == 0;
        let intro = self.intro();
        ctx.say(intro);
        if self.high_water {
            ctx.say_queued(events::HIGH_WATER_LINE);
        }
        // The full controls until a night is finished; then F1 has them.
        if Progress::load(ctx).nights == 0 {
            let c = self.controls(ctx);
            ctx.say_queued(c);
        } else {
            ctx.say_queued("F1 lists the controls.");
        }
        let n = self.night();
        let first = format!(
            "Landing 1 of {}. {}",
            n.rules.landings,
            self.first_roll_words(ctx)
        );
        ctx.say_queued(first);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.stop_band(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended {
            return;
        }
        if self.band.is_none_or(|b| !ctx.audio.is_playing(b)) {
            self.band = Some(sound::saloon(ctx, self.high_water));
        }
        ctx.say(format!(
            "Back at the table. {}",
            self.status_layers().remove(0)
        ));
    }

    fn handle_input(&mut self, ctx: &mut Ctx<Parlor>, event: &InputEvent) {
        if self.ended {
            return;
        }
        let action = ctx.action_for(event);
        if action.is_none() && event.pressed() == Some(Key::Escape) {
            if event.is_repeat() {
                return;
            }
            let leave = if self.is_challenge() {
                Leave::Challenge(CHALLENGES_NAME)
            } else {
                Leave::Title(TITLE_NAME)
            };
            menus::pause(ctx, HELP, leave);
            return;
        }
        if event.is_repeat() {
            return;
        }
        if action.is_some() && action != Some(ParlorAction::Secondary) {
            self.confirming = false;
        }
        match action {
            Some(ParlorAction::Primary | ParlorAction::Secondary) if self.table.busy() => {
                self.skip(ctx)
            }
            Some(ParlorAction::Primary) => self.roll(ctx),
            Some(ParlorAction::Secondary) => self.bank(ctx),
            Some(ParlorAction::Left | ParlorAction::Right) => {
                self.look(ctx, action == Some(ParlorAction::Right))
            }
            Some(ParlorAction::Up) => {
                let text = odds_words(&self.rules());
                ctx.say(text);
            }
            Some(ParlorAction::Down) => {
                let text = format!("Night totals: {}.", self.night().standings());
                ctx.say(text);
            }
            Some(ParlorAction::Status) => {
                let layers = self.status_layers();
                self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => {
                let text = self.last_roll_words(ctx);
                ctx.say(text);
            }
            Some(ParlorAction::Hint) => {
                let text = self.hint_words(ctx);
                ctx.say(text);
            }
            None => {}
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.ended {
            return;
        }
        let held = self.hold.get();
        if held > 0.0 {
            self.hold.set((held - dt).max(0.0));
            return;
        }
        let cues = self.table.update(dt);
        self.play_cues(ctx, cues);
        self.check_end(ctx);
    }

    fn lines(&self, _ctx: &Ctx<Parlor>) -> Vec<String> {
        let n = self.night();
        let mut out = vec![match &self.job {
            Some(job) => format!(
                "{}, night {} of {}",
                challenge::name(job.period),
                job.index + 1,
                job.nights()
            ),
            None if n.rules.moon => format!("Riverboat Dice: {}", events::MOON_NAME),
            None => "Riverboat Dice: Saloon".to_string(),
        }];
        out.push(format!(
            "Landing {} of {}, the table holds {}",
            n.landing, n.rules.landings, n.pot
        ));
        for s in &n.seats {
            out.push(format!(
                "{}: {}, {}",
                sentence_start(s.who.name()),
                s.night,
                if s.seated { "seated" } else { "standing" }
            ));
        }
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let mut now = format!(
            "{} Stay in while you dare, and bank before a 1 comes.",
            self.status()
        );
        if let Some(job) = &self.job {
            now.push_str(&format!(
                " This is the {} challenge, {}. Leaving from the pause menu does not count it.",
                job.period.label().to_lowercase(),
                challenge::name(job.period)
            ));
        }
        format!("{now} {}", self.controls(ctx))
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

mod visuals_tablescreen;
