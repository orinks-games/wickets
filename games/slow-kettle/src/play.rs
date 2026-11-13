//! The counter: guests come in, you choose a tea and a cup, heat the
//! kettle, pour, steep, and hear how the cup went.

use std::collections::HashSet;

use hearken::text::sentence_start;
use hearken::{Ctx, InputEvent, Key, State};
use parlor_core::menus::{self, labels, Back, Leave, Results};
use parlor_core::{challenges, Challenge, Date, Pages, Parlor, ParlorAction, Period, Records, Rng};

use crate::challenge::{self, CHALLENGES_NAME};
use crate::days::{coming, occasion_on, Occasion};
use crate::guests::{regulars_met, Grade, Regular};
use crate::rules::{
    self, judge, plan, practice_visit, strength_words, Brewed, Cue, Mode, Phase, Service, Sitting,
    Stage, Verdict, Visit, Who, EVENING_AFTER,
};
use crate::save::House;
use crate::shop::Shop;
use crate::sound::{self, KettleSong};
use crate::talk::{self, coins_words, cups_words, said};
use crate::teas::{Cup, Tea, Tune};
use crate::{HELP, ID, TITLE_NAME};

/// The play screen's `State::name`.
pub const COUNTER_NAME: &str = "slow_kettle_counter";

/// Cues explained in words the first time they play.
const TAUGHT_SONG: &str = "song";
const TAUGHT_TICK: &str = "tick";
const TAUGHT_TIMER: &str = "timer";
const TAUGHT_NEXT: &str = "next";

pub struct Counter {
    pub mode: Mode,
    pub sitting: Sitting,
    pub service: Service,
    pub shelf: Vec<&'static Tea>,
    pub cups: Vec<&'static Cup>,
    pub shelf_at: usize,
    pub cup_at: usize,
    pub today: Date,
    rng: Rng,
    song: KettleSong,
    challenge: Option<Challenge>,
    taught: HashSet<&'static str>,
    last_prompt: String,
    quiet: bool,
    tune: Option<&'static Tune>,
    regulars: Vec<&'static Regular>,
    ended: bool,
    started: bool,
    /// S, in layers: a second press reads the rest.
    status_key: parlor_core::StatusKey,
}

impl Counter {
    /// A sitting of `mode`, planned from the saved house and today.
    pub fn new(ctx: &mut Ctx<Parlor>, mode: Mode) -> Counter {
        let today = challenges::today(ctx);
        let house = House::load(ctx);
        let mut rng = rules::sitting_rng();
        let visits = plan(&house.known(), today, mode, &mut rng);
        let shelf = rules::shelf_for(&|id| house.owns_tea(id), today);
        Counter {
            status_key: Default::default(),
            mode,
            sitting: Sitting::new(mode, visits),
            service: Service::new(house.sand_timer),
            shelf,
            cups: house.cupboard(),
            shelf_at: 0,
            cup_at: 0,
            today,
            rng,
            song: KettleSong::default(),
            challenge: None,
            taught: HashSet::new(),
            last_prompt: String::new(),
            quiet: house.quiet,
            tune: house.playing(),
            regulars: regulars_met(house.sessions()),
            ended: false,
            started: false,
        }
    }

    /// The challenge's guests, from its seed, on the whole shelf.
    pub fn challenge(ctx: &mut Ctx<Parlor>, ch: Challenge) -> Counter {
        let mode = Mode::Challenge(ch.period);
        let mut counter = Counter::new(ctx, mode);
        let mut rng = ch.rng();
        let visits = plan(&Default::default(), counter.today, mode, &mut rng);
        counter.sitting = Sitting::new(mode, visits);
        counter.shelf = rules::challenge_shelf();
        counter.cups = rules::all_cups();
        counter.service = Service::new(false);
        counter.rng = rng;
        counter.challenge = Some(ch);
        counter
    }

    pub fn is_challenge(&self) -> bool {
        self.challenge.is_some()
    }

    /// What the main action does once a cup is served: the next guest,
    /// or, after the last one, closing up.
    fn next_words(&self) -> &'static str {
        if self.sitting.last() {
            "closes the tea house"
        } else {
            "welcomes the next guest"
        }
    }

    fn practice(&self) -> bool {
        self.mode == Mode::Practice
    }

    pub fn visit(&self) -> Visit {
        if self.practice() {
            practice_visit(self.today.year)
        } else {
            self.sitting.visit().clone()
        }
    }

    pub fn tea(&self) -> &'static Tea {
        self.shelf[self.shelf_at.min(self.shelf.len() - 1)]
    }

    pub fn cup(&self) -> &'static Cup {
        self.cups[self.cup_at.min(self.cups.len() - 1)]
    }

    fn bed(&self) -> &'static str {
        if self.mode == Mode::Evening {
            sound::RAIN
        } else {
            sound::HOUSE
        }
    }

    fn intro(&self) -> String {
        let n = self.sitting.visits.len();
        let head = match (self.mode, &self.challenge) {
            (_, Some(ch)) => format!("{}: {}.", ch.title(), challenge::name(ch.period)),
            (Mode::Evening, _) => {
                format!("A long, rainy evening. {n} guests are on their way.")
            }
            (Mode::Practice, _) => {
                "Practice brewing. Brew any tea for yourself, with the exact heat and steep read back."
                    .to_string()
            }
            _ => format!("The tea house is open. {n} guests this afternoon."),
        };
        if self.is_challenge() || self.practice() {
            return head;
        }
        let blend = crate::days::blend(self.today);
        let mut out = format!("{head} {} is on the shelf.", blend.name);
        if let Some(o) = occasion_on(self.today) {
            out.push_str(&format!(" A special guest is due today, for {}.", o.name()));
        }
        out
    }

    fn controls(&self, ctx: &Ctx<Parlor>) -> String {
        let u = ctx.control_name(ParlorAction::Up);
        let d = ctx.control_name(ParlorAction::Down);
        let l = ctx.control_name(ParlorAction::Left);
        let r = ctx.control_name(ParlorAction::Right);
        let p = ctx.control_name(ParlorAction::Primary);
        let t = ctx.control_name(ParlorAction::Secondary);
        let s = ctx.control_name(ParlorAction::Status);
        let rep = ctx.control_name(ParlorAction::Repeat);
        let h = ctx.control_name(ParlorAction::Hint);
        format!(
            "{} and {d} move along the shelf, {l} and {r} choose a cup. {p} chooses, heats, pours and lifts the leaves. {t} says more about what is in front of you. {s} gives the status, twice for more, {rep} repeats, {h} gives a hint. F1 helps, Escape pauses.",
            sentence_start(&u)
        )
    }

    fn prompt(&mut self, ctx: &mut Ctx<Parlor>, text: String) {
        ctx.say(text.clone());
        self.last_prompt = text;
    }

    /// The door bell and footsteps: the next guest is coming in.
    fn welcome(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.practice() {
            self.service.arrive(1);
            self.service.to_shelf();
            let v = self.visit();
            let text = format!("{} {}", v.asks, self.shelf_words());
            self.prompt(ctx, text);
            return;
        }
        let v = self.visit();
        self.service.arrive(v.cups);
        sound::play(ctx, sound::BELL);
        match v.who {
            Who::Holiday(Occasion::Halloween) => {
                sound::play(ctx, sound::STRANGER);
            }
            Who::Holiday(_) => {
                sound::play(ctx, sound::HOLIDAY);
                sound::play(ctx, sound::STEPS);
            }
            _ => {
                sound::play(ctx, sound::STEPS);
            }
        }
        if let Some(tea) = v.wants {
            if let Some(at) = self.shelf.iter().position(|t| t.id == tea.id) {
                self.shelf_at = at;
            }
        }
    }

    /// The guest is at the counter and says what they would like.
    fn arrived(&mut self, ctx: &mut Ctx<Parlor>) {
        let v = self.visit();
        let text = talk::arrival(&v, &self.regulars);
        self.prompt(ctx, text);
        // The results page gives the countdown; the regular just passes
        // on the news.
        if let (Some(o), Who::Regular(r)) = (v.hint, v.who) {
            ctx.say_queued(format!("{}: {}", r.name, o.hint()));
        }
        let shelf = self.shelf_words();
        ctx.say_queued(shelf);
    }

    /// "Shelf: Sencha. A Japanese green tea, grassy and bright."
    fn shelf_words(&self) -> String {
        let t = self.tea();
        format!(
            "Shelf: {}, {} of {}. {}",
            t.name,
            self.shelf_at + 1,
            self.shelf.len(),
            t.about
        )
    }

    fn move_shelf(&mut self, ctx: &mut Ctx<Parlor>, down: bool) {
        let n = self.shelf.len();
        self.shelf_at = if down {
            (self.shelf_at + 1) % n
        } else {
            (self.shelf_at + n - 1) % n
        };
        sound::play(ctx, sound::SHELF);
        let t = self.tea();
        ctx.say(format!("{}. {}", t.name, t.about));
    }

    fn move_cup(&mut self, ctx: &mut Ctx<Parlor>, right: bool) {
        let n = self.cups.len();
        if n < 2 {
            sound::play(ctx, sound::NOPE);
            ctx.say("Only the plain white cup so far. The shop has more.");
            return;
        }
        self.cup_at = if right {
            (self.cup_at + 1) % n
        } else {
            (self.cup_at + n - 1) % n
        };
        let c = self.cup();
        sound::play(ctx, sound::cup(c));
        ctx.say(format!("{}. {}", c.name, c.about));
    }

    fn choose(&mut self, ctx: &mut Ctx<Parlor>) {
        let tea = self.tea();
        let cup = self.cup();
        self.service.choose(tea);
        sound::play(ctx, sound::TIN);
        let p = ctx.control_name(ParlorAction::Primary);
        let s = ctx.control_name(ParlorAction::Status);
        let mut text = format!(
            "{} in the pot, {} ready. {} puts the kettle on.",
            tea.name,
            cup.name.to_lowercase(),
            sentence_start(&p)
        );
        if self.taught.insert(TAUGHT_SONG) {
            text.push_str(&format!(
                " {} says the heat this tea wants.",
                sentence_start(&s)
            ));
        }
        self.prompt(ctx, text);
    }

    fn heat(&mut self, ctx: &mut Ctx<Parlor>) {
        self.service.heat();
        sound::play(ctx, sound::KETTLE_ON);
        self.song.start(ctx, self.service.kettle.temp);
        ctx.say("Kettle on.");
    }

    fn pour(&mut self, ctx: &mut Ctx<Parlor>) {
        let at = self.service.pour();
        self.song.stop(ctx);
        sound::play(ctx, sound::POUR);
        let p = ctx.control_name(ParlorAction::Primary);
        let mut text = format!("Poured at {} degrees. Steeping.", at.round() as u32);
        if self.taught.insert(TAUGHT_TICK) {
            text.push_str(&format!(
                " {} lifts the leaves out when it tastes right.",
                sentence_start(&p)
            ));
        }
        self.prompt(ctx, text);
    }

    fn lift(&mut self, ctx: &mut Ctx<Parlor>) {
        self.service.lift();
        sound::play(ctx, sound::LIFT);
        self.after_lift(ctx, false);
    }

    fn after_lift(&mut self, ctx: &mut Ctx<Parlor>, long: bool) {
        let secs = self.service.seconds();
        let head = if long {
            "The leaves came out on their own.".to_string()
        } else {
            format!("Leaves out after {}.", talk::seconds_words(secs))
        };
        if self.service.phase == Phase::Pouring {
            let p = ctx.control_name(ParlorAction::Primary);
            let text = format!(
                "{head} {} pours each cup. {} to pour.",
                sentence_start(&p),
                cups_words(self.service.cups as usize)
            );
            self.prompt(ctx, text);
        } else {
            ctx.say(head);
            self.serve(ctx);
        }
    }

    fn pour_cup(&mut self, ctx: &mut Ctx<Parlor>) {
        let n = self.service.pour_cup();
        sound::play(ctx, sound::CUP_POUR);
        let left = self.service.cups - n;
        if self.service.phase == Phase::Served {
            ctx.say(format!("{} cup poured.", talk::ordinal(n)));
            self.serve(ctx);
        } else {
            let text = format!(
                "{} cup poured. {} to go.",
                talk::ordinal(n),
                cups_words(left as usize)
            );
            self.prompt(ctx, text);
        }
    }

    /// The guest has their cup: how it went, the coins, and any story.
    fn serve(&mut self, ctx: &mut Ctx<Parlor>) {
        let v = self.visit();
        let tea = self.service.tea.unwrap_or(self.tea());
        let cup = self.cup();
        let brewed = Brewed {
            tea,
            cup,
            heat: self.service.poured_at,
            steep: self.service.steeped,
            taste: v.taste,
            wants: v.wants,
        };
        let verdict = judge(&brewed);
        if rules::iced(tea) {
            sound::play(ctx, sound::ICE);
        }
        sound::play(ctx, sound::cup(cup));
        sound::play(
            ctx,
            match verdict.grade {
                Grade::Perfect => sound::PERFECT,
                Grade::Lovely => sound::LOVELY,
                _ => sound::KIND,
            },
        );
        if self.practice() {
            self.practice_words(ctx, tea, &verdict);
            return;
        }
        let (pay, tip) = self.sitting.serve(verdict, v.cups);
        // Challenge cups earn points, not coins.
        if !self.is_challenge() {
            sound::play(ctx, sound::COINS);
        }
        let mut first = said(&v.name(), rules::line(&mut self.rng, verdict.grade.lines()));
        if let Some(slip) = verdict.slip(tea) {
            first.push(' ');
            first.push_str(slip.words());
        }
        let mut money = if self.is_challenge() {
            format!("{} points.", verdict.points)
        } else if tip > 0 {
            format!(
                "{} and a tip of {}.",
                sentence_start(&coins_words(pay)),
                coins_words(tip)
            )
        } else {
            format!("{}.", sentence_start(&coins_words(pay)))
        };
        if verdict.grade == Grade::Perfect && !self.is_challenge() {
            let note = rules::note(&mut self.rng);
            money.push_str(&format!(" A note on the saucer: {note}"));
        }
        let tail = self.save_cup(ctx, &v, tea, &verdict, pay + tip);
        if self.taught.insert(TAUGHT_NEXT) {
            let p = ctx.control_name(ParlorAction::Primary);
            money.push_str(&format!(" {} {}.", sentence_start(&p), self.next_words()));
        }
        ctx.say(first.clone());
        ctx.say_queued(money.clone());
        let mut all = format!("{first} {money}");
        if let Some(tail) = tail {
            ctx.say_queued(tail.clone());
            all.push(' ');
            all.push_str(&tail);
        }
        self.last_prompt = all;
    }

    fn practice_words(&mut self, ctx: &mut Ctx<Parlor>, tea: &Tea, v: &Verdict) {
        let mut house = House::load(ctx);
        house.practice_cups += 1;
        house.store(ctx);
        let heat = self.service.poured_at.round() as u32;
        let secs = self.service.steeped.round() as u32;
        let p = ctx.control_name(ParlorAction::Primary);
        let text = format!(
            "A {} cup, {} points. Poured at {heat} degrees, {} wants {}. Steeped {}, it likes {}. {} brews another.",
            v.grade.word(),
            v.points,
            tea.name,
            tea.heat,
            talk::seconds_words(secs),
            tea.steep,
            sentence_start(&p)
        );
        self.prompt(ctx, text);
    }

    /// Saves a served cup: counts, coins, the regular's story and any
    /// keepsake. Returns what the guest says next, if anything.
    fn save_cup(
        &mut self,
        ctx: &mut Ctx<Parlor>,
        v: &Visit,
        tea: &Tea,
        verdict: &Verdict,
        coins: u32,
    ) -> Option<String> {
        let mut house = House::load(ctx);
        house.cups_served += v.cups as u32;
        if verdict.grade == Grade::Perfect {
            house.perfect += 1;
        }
        house.tasted.insert(tea.id.to_string());
        if self.is_challenge() {
            house.store(ctx);
            return None;
        }
        house.coins += coins;
        house.coins_earned += coins;
        let tail = match v.who {
            Who::Regular(r) => {
                let told = house.chapters.get(r.id).copied().unwrap_or(0);
                house.last_seen.insert(r.id.to_string(), house.sessions());
                let words = match r.story.get(told as usize) {
                    Some(part) => {
                        house.chapters.insert(r.id.to_string(), told + 1);
                        *part
                    }
                    None => self.rng.pick(&r.after),
                };
                Some(format!("{}: {words}", r.name))
            }
            Who::Holiday(o) => Some(self.keepsake(ctx, &mut house, o, v.year)),
            _ => None,
        };
        house.store(ctx);
        tail
    }

    fn keepsake(
        &mut self,
        ctx: &mut Ctx<Parlor>,
        house: &mut House,
        o: Occasion,
        year: i32,
    ) -> String {
        let words = talk::farewell(o, year, &self.regulars);
        if house.keep(o, year) {
            sound::play(ctx, sound::KEEPSAKE);
            format!("{words} It goes on your keepsake shelf.")
        } else {
            format!("{words} You already have one from this year, so you give it to the next guest who needs some luck.")
        }
    }

    fn next_guest(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.practice() {
            self.welcome(ctx);
            return;
        }
        if self.sitting.advance() {
            ctx.say("The next guest.");
            self.welcome(ctx);
        } else {
            self.close(ctx);
        }
    }

    /// The sitting is over: records, results.
    fn close(&mut self, ctx: &mut Ctx<Parlor>) {
        self.ended = true;
        self.song.stop(ctx);
        ctx.audio.set_bed(None);
        ctx.audio.play_music(Some(sound::CLOSING), false);
        if let Some(ch) = self.challenge.take() {
            let pages = challenge::finish(ctx, &ch, &self.sitting);
            let summary = format!("{} result: {} points", ch.title(), self.sitting.points());
            let mut results = Some(after_challenge(ch.period, summary, pages.clone()));
            ctx.replace(Pages::new(pages).then(move |ctx| {
                if let Some(results) = results.take() {
                    ctx.replace(results);
                }
            }));
            return;
        }
        let pages = self.close_house(ctx);
        let summary = format!(
            "Closed. {}, {} perfect, {}",
            cups_words(self.sitting.cups as usize),
            self.sitting.perfect(),
            coins_words(self.sitting.coins)
        );
        let mut results = Some(after_sitting(self.mode, summary, pages.clone()));
        ctx.push(Pages::new(pages).then(move |ctx| {
            if let Some(results) = results.take() {
                ctx.pop_to_quiet(TITLE_NAME);
                ctx.push(results);
            }
        }));
    }

    fn close_house(&mut self, ctx: &mut Ctx<Parlor>) -> Vec<String> {
        let mut house = House::load(ctx);
        let was_evening_locked = house.afternoons < EVENING_AFTER;
        let newcomers_before = regulars_met(house.sessions()).len();
        match self.mode {
            Mode::Evening => house.evenings += 1,
            _ => house.afternoons += 1,
        }
        house.store(ctx);
        let coins = self.sitting.coins;
        let best = self
            .mode
            .board()
            .map(|b| Records::record(ctx, ID, b, coins as u64));
        let when = if self.mode == Mode::Evening {
            "the evening"
        } else {
            "the afternoon"
        };
        let mut first = format!(
            "Closed for {when}. {} served, {} perfect, {} earned.",
            cups_words(self.sitting.cups as usize),
            self.sitting.perfect(),
            coins_words(coins)
        );
        match best {
            Some(parlor_core::ScoreResult::NewBest { previous }) => {
                first.push_str(&format!(" A new best, beating {previous}."))
            }
            Some(parlor_core::ScoreResult::Short { best }) => {
                first.push_str(&format!(" Your best is {best}."))
            }
            _ => {}
        }
        let mut second = format!(
            "You have {}. {} discovered.",
            coins_words(house.coins),
            sentence_start(&house.tasted_words())
        );
        if was_evening_locked && house.afternoons >= EVENING_AFTER {
            second.push_str(" The long evening is open on the title.");
        }
        let newcomers = regulars_met(house.sessions());
        if newcomers.len() > newcomers_before {
            if let Some(r) = newcomers.last() {
                second.push_str(&format!(
                    " Word is getting round: {} may drop in next time.",
                    r.name
                ));
            }
        }
        if let Some((o, _)) = coming(self.today) {
            if let Some(c) = o.countdown(self.today) {
                second.push_str(&format!(" {c}"));
            }
        }
        vec![first, second]
    }

    /// The status line, in a fixed order: the guest, what they would like,
    /// where the cup is, then your coins.
    /// The status key, in layers. First: what the pot, kettle or cups are
    /// doing now, and what the guest wants while you choose. A second
    /// press: which guest this is, what they would like, and your coins.
    pub fn status_layers(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        let v = self.visit();
        let mut first = Vec::new();
        let mut more = Vec::new();
        if !self.practice() {
            more.push(format!(
                "Guest {} of {}",
                self.sitting.index + 1,
                self.sitting.visits.len()
            ));
            more.push(format!(
                "{} would like {}",
                mid_sentence(&v.name()),
                talk::want_words(&v)
            ));
        }
        let s = &self.service;
        match s.phase {
            Phase::Arriving => first.push("a guest is coming in".to_string()),
            Phase::Choosing => {
                first.push(format!("shelf: {}", self.tea().name));
                first.push(format!("cup: {}", self.cup().name.to_lowercase()));
                if !self.practice() {
                    first.push(format!(
                        "{} would like {}",
                        mid_sentence(&v.name()),
                        talk::want_words(&v)
                    ));
                    more.pop();
                }
            }
            Phase::Ready => {
                let t = s.tea.unwrap_or(self.tea());
                first.push(format!("{} in the pot, kettle cold", t.name));
                first.push(format!("it wants {}", t.heat_words()));
            }
            Phase::Heating => {
                let t = s.tea.unwrap_or(self.tea());
                first.push(format!(
                    "kettle at {} degrees, {}",
                    s.kettle.degrees(),
                    s.kettle.stage().words()
                ));
                first.push(format!("{} wants {}", t.name, t.heat_words()));
            }
            Phase::Steeping => {
                let t = s.tea.unwrap_or(self.tea());
                first.push(format!(
                    "steeping {}, {}",
                    talk::seconds_words(s.seconds()),
                    strength_words(s.steeped, t.steep as f64)
                ));
                first.push(format!("{} likes {} seconds", t.name, t.steep));
            }
            Phase::Pouring => first.push(format!("{} of {} cups poured", s.poured, s.cups)),
            Phase::Served => {
                let p = ctx.control_name(ParlorAction::Primary);
                first.push(format!("cup served, {p} {}", self.next_words()));
            }
        }
        if !self.is_challenge() && !self.practice() {
            more.push(format!("{} earned", coins_words(self.sitting.coins)));
        }
        if self.is_challenge() {
            more.push(format!("{} points", self.sitting.points()));
        }
        let line = |parts: Vec<String>| {
            if parts.is_empty() {
                String::new()
            } else {
                format!("{}.", sentence_start(&parts.join(", ")))
            }
        };
        vec![line(first), line(more)]
    }

    /// Every layer of the status key at once.
    pub fn status(&self, ctx: &Ctx<Parlor>) -> String {
        parlor_core::status::sentences(self.status_layers(ctx))
    }

    fn hint(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::play(ctx, sound::HINT);
        let v = self.visit();
        let p = ctx.control_name(ParlorAction::Primary);
        let s = &self.service;
        let text = match s.phase {
            Phase::Arriving => "A guest is coming in.".to_string(),
            Phase::Choosing => match (v.wants, v.taste) {
                (Some(t), _) => match self.shelf.iter().position(|x| x.id == t.id) {
                    Some(at) => format!(
                        "{} would like {}. It is number {} on your shelf.",
                        v.name(),
                        t.name,
                        at + 1
                    ),
                    None => format!("{} would like {}.", v.name(), t.name),
                },
                (None, Some(taste)) => {
                    let fits: Vec<&str> = rules::suggestions(&self.shelf, taste)
                        .iter()
                        .map(|t| t.name)
                        .collect();
                    let cups: Vec<String> = self
                        .cups
                        .iter()
                        .filter(|c| c.suits(taste))
                        .map(|c| c.name.to_lowercase())
                        .collect();
                    let mut out = if fits.is_empty() {
                        format!(
                            "Nothing on your shelf is {}. Any tea still pleases.",
                            taste.word()
                        )
                    } else {
                        format!("For something {}: {}.", taste.word(), fits.join(", "))
                    };
                    if let Some(c) = cups.first() {
                        out.push_str(&format!(" The {c} suits it too."));
                    }
                    out
                }
                _ => "Any tea you like.".to_string(),
            },
            Phase::Ready => {
                let t = s.tea.unwrap_or(self.tea());
                format!(
                    "{} {} puts the kettle on.",
                    talk::heat_hint(t),
                    sentence_start(&p)
                )
            }
            Phase::Heating => talk::heat_hint(s.tea.unwrap_or(self.tea())),
            Phase::Steeping => {
                let t = s.tea.unwrap_or(self.tea());
                format!(
                    "{} likes {} seconds. You are at {}. {} lifts the leaves.",
                    t.name,
                    t.steep,
                    s.seconds(),
                    sentence_start(&p)
                )
            }
            Phase::Pouring => format!("{} pours the next cup.", sentence_start(&p)),
            Phase::Served => format!("{} {}.", sentence_start(&p), self.next_words()),
        };
        ctx.say(text);
    }

    /// The second action: more about what is in front of you.
    fn closer_look(&mut self, ctx: &mut Ctx<Parlor>) {
        let s = &self.service;
        let text = match s.phase {
            Phase::Arriving => "A guest is coming to the counter.".to_string(),
            Phase::Choosing => {
                let t = self.tea();
                let tastes: Vec<&str> = t.tastes.iter().map(|x| x.word()).collect();
                format!(
                    "{} {} It tastes {}.",
                    t.about,
                    t.needs(),
                    tastes.join(" and ")
                )
            }
            Phase::Ready => s.tea.unwrap_or(self.tea()).needs(),
            Phase::Heating => format!(
                "The kettle: {}, {} degrees.",
                s.kettle.stage().words(),
                s.kettle.degrees()
            ),
            Phase::Steeping => {
                let t = s.tea.unwrap_or(self.tea());
                let secs = talk::seconds_words(s.seconds());
                format!(
                    "You breathe in the steam: {}, {secs}.",
                    strength_words(s.steeped, t.steep as f64)
                )
            }
            Phase::Pouring => format!("{} of {} cups poured.", s.poured, s.cups),
            Phase::Served => self.last_prompt.clone(),
        };
        ctx.say(text);
    }

    fn primary(&mut self, ctx: &mut Ctx<Parlor>) {
        match self.service.phase {
            Phase::Arriving => {
                self.service.skip_arrival();
                self.arrived(ctx);
            }
            Phase::Choosing => self.choose(ctx),
            Phase::Ready => self.heat(ctx),
            Phase::Heating => self.pour(ctx),
            Phase::Steeping => self.lift(ctx),
            Phase::Pouring => self.pour_cup(ctx),
            Phase::Served => self.next_guest(ctx),
        }
    }

    fn arrows(&mut self, ctx: &mut Ctx<Parlor>, action: ParlorAction) {
        let vertical = matches!(action, ParlorAction::Up | ParlorAction::Down);
        match self.service.phase {
            Phase::Arriving => {
                self.service.skip_arrival();
                self.arrived(ctx);
            }
            Phase::Choosing if vertical => self.move_shelf(ctx, action == ParlorAction::Down),
            Phase::Choosing => self.move_cup(ctx, action == ParlorAction::Right),
            Phase::Ready if vertical => {
                self.service.to_shelf();
                sound::play(ctx, sound::SHELF);
                let text = format!("Back to the shelf. {}", self.shelf_words());
                self.prompt(ctx, text);
            }
            _ => {
                sound::play(ctx, sound::NOPE);
                let p = ctx.control_name(ParlorAction::Primary);
                let what = match self.service.phase {
                    Phase::Ready => "puts the kettle on",
                    Phase::Heating => "pours",
                    Phase::Steeping => "lifts the leaves",
                    Phase::Pouring => "pours the next cup",
                    _ => self.next_words(),
                };
                // Once the cup is served, the tea is no longer on its way.
                let lead = match self.service.phase {
                    Phase::Ready | Phase::Heating | Phase::Steeping | Phase::Pouring => {
                        "The tea is on its way. "
                    }
                    _ => "",
                };
                ctx.say(format!("{lead}{} {what}.", sentence_start(&p)));
            }
        }
    }

    fn cues(&mut self, ctx: &mut Ctx<Parlor>, cues: Vec<Cue>) {
        for cue in cues {
            match cue {
                Cue::Arrived => self.arrived(ctx),
                Cue::Stage(stage) => {
                    if !self.quiet && stage != Stage::Quiet {
                        ctx.say(stage.call_out());
                    }
                }
                Cue::Tick(n) => {
                    sound::play(ctx, sound::TICK);
                    if !self.quiet && n % 5 == 0 {
                        ctx.say(format!("{n} seconds."));
                    }
                }
                Cue::Timer => {
                    sound::play(ctx, sound::TIMER);
                    if self.taught.insert(TAUGHT_TIMER) {
                        ctx.say("Sand timer: steeped.");
                    } else if !self.quiet {
                        ctx.say("Sand timer.");
                    }
                }
                Cue::LongSteep => {
                    sound::play(ctx, sound::LIFT);
                    self.after_lift(ctx, true);
                }
            }
        }
    }
}

/// The results after an afternoon or an evening.
pub fn after_sitting(mode: Mode, summary: String, pages: Vec<String>) -> Results {
    let help = match mode {
        Mode::Evening => "Another long evening, with new guests.",
        _ => "Another afternoon, with new guests.",
    };
    Results::new(summary, pages, Back::Title(TITLE_NAME))
        .again(labels::PLAY_AGAIN, help, move |ctx| {
            let counter = Counter::new(ctx, mode);
            ctx.replace(counter)
        })
        // Not "Shop": a row starting with S would take the status key, which
        // repeats the summary on every results menu.
        .extra(
            "Visit the shop",
            "Spend your coins on teas, cups and the music box.",
            |ctx| ctx.push(Shop::new()),
        )
}

/// The results after a challenge.
pub fn after_challenge(period: Period, summary: String, pages: Vec<String>) -> Results {
    Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME)).again(
        labels::challenge_again(period),
        "The same guests, straight away. Only your best result is kept.",
        move |ctx| {
            let ch = Challenge::current(ctx, ID, period);
            let counter = Counter::challenge(ctx, ch);
            ctx.replace(counter)
        },
    )
}

impl State<Parlor> for Counter {
    fn visuals(&self, ctx: &Ctx<Parlor>) -> hearken::visuals::Scene {
        self.visual_scene(ctx)
    }

    fn name(&self) -> &'static str {
        COUNTER_NAME
    }

    fn enter(&mut self, ctx: &mut Ctx<Parlor>) {
        sound::house(ctx, self.bed(), self.tune);
        let intro = self.intro();
        ctx.say(intro);
        let c = self.controls(ctx);
        ctx.say_queued(c);
        self.started = true;
        self.welcome(ctx);
    }

    fn exit(&mut self, ctx: &mut Ctx<Parlor>) {
        self.song.stop(ctx);
        ctx.audio.set_bed(None);
    }

    fn resume(&mut self, ctx: &mut Ctx<Parlor>) {
        if self.ended {
            return;
        }
        // Settings, from the pause menu, may have changed the call-outs or
        // the music box's tune.
        let house = House::load(ctx);
        self.quiet = house.quiet;
        self.tune = house.playing();
        sound::house(ctx, self.bed(), self.tune);
        let s = self.status_layers(ctx).remove(0);
        ctx.say(format!("Back at the counter. {s}"));
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
        let arrow = matches!(
            action,
            Some(ParlorAction::Left | ParlorAction::Right | ParlorAction::Up | ParlorAction::Down)
        );
        if event.is_repeat() && !(arrow && self.service.phase == Phase::Choosing) {
            return;
        }
        if !matches!(
            event,
            InputEvent::KeyDown { .. } | InputEvent::ControllerButtonDown { .. }
        ) {
            return;
        }
        match action {
            Some(
                a @ (ParlorAction::Left
                | ParlorAction::Right
                | ParlorAction::Up
                | ParlorAction::Down),
            ) => self.arrows(ctx, a),
            Some(ParlorAction::Primary) => self.primary(ctx),
            Some(ParlorAction::Secondary) => self.closer_look(ctx),
            Some(ParlorAction::Status) => {
                let layers = self.status_layers(ctx);
                self.status_key.speak(ctx, layers);
            }
            Some(ParlorAction::Repeat) => {
                if self.last_prompt.is_empty() {
                    let s = self.status_layers(ctx).remove(0);
                    ctx.say(s);
                } else {
                    ctx.say(self.last_prompt.clone());
                }
            }
            Some(ParlorAction::Hint) => self.hint(ctx),
            None => {}
        }
    }

    fn update(&mut self, ctx: &mut Ctx<Parlor>, dt: f64) {
        if self.ended || !self.started {
            return;
        }
        let cues = self.service.update(dt);
        if self.song.playing() {
            let t = self.service.kettle.temp;
            self.song.follow(ctx, t);
        }
        self.cues(ctx, cues);
    }

    fn lines(&self, ctx: &Ctx<Parlor>) -> Vec<String> {
        let mut out = vec![match &self.challenge {
            Some(ch) => format!("Slow Kettle: {}", challenge::name(ch.period)),
            None => format!("Slow Kettle: {}", self.mode.name()),
        }];
        out.push(self.status(ctx));
        out.push(self.last_prompt.clone());
        out
    }

    fn help(&self, ctx: &Ctx<Parlor>) -> String {
        let p = ctx.control_name(ParlorAction::Primary);
        let now = match self.service.phase {
            Phase::Arriving => "A guest is coming in.".to_string(),
            Phase::Choosing => "Choose a tea for the guest's taste, and a cup.".to_string(),
            Phase::Ready => format!("{} puts the kettle on.", sentence_start(&p)),
            Phase::Heating => format!(
                "{} pours when the kettle is right for this tea.",
                sentence_start(&p)
            ),
            Phase::Steeping => format!(
                "{} lifts the leaves out when it tastes right.",
                sentence_start(&p)
            ),
            Phase::Pouring => format!("{} pours each cup.", sentence_start(&p)),
            // The status already says what welcomes the next guest.
            Phase::Served => String::new(),
        };
        let mut text =
            format!("{} {now} Nothing here can go wrong.", self.status(ctx)).replace("  ", " ");
        if let Some(ch) = &self.challenge {
            text.push_str(&format!(
                " This is the {} challenge. Leaving from the pause menu does not count it.",
                ch.period.label().to_lowercase()
            ));
        }
        format!("{text} {}", self.controls(ctx))
    }

    /// F1 during play: a long explanation is read as pages, with the game
    /// held still until they close.
    fn help_pressed(&mut self, ctx: &mut Ctx<Parlor>) {
        let help = self.help(ctx);
        parlor_core::explain(ctx, help);
    }
}

/// The long evening's row is open after this many afternoons.
pub fn evening_open(house: &House) -> bool {
    house.afternoons >= EVENING_AFTER
}

/// A guest's name inside a status line: "The builder" reads "the builder"
/// after a comma; a given name stays as it is.
fn mid_sentence(name: &str) -> String {
    match name.strip_prefix("The ") {
        Some(rest) => format!("the {rest}"),
        None => name.to_string(),
    }
}

mod visuals_counter;
