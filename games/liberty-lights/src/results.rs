//! How the crowd took the show, what it scored, and where to go next, on
//! the shared results menu.

use hearken::text::sentence_start;
use hearken::{Bus, Ctx};
use parlor_core::menus::{labels, Back, Results};
use parlor_core::words::streak_sentence;
use parlor_core::{Challenges, Parlor, Records, ScoreResult};

use crate::festival::Run;
use crate::independence::{self, Keepsakes, Kept, NightRun};
use crate::progress::{self, Progress};
use crate::replay::Replay;
use crate::rules::{Judgement, Launch, PASS_PERCENT, TOTAL_LEVELS};
use crate::show::{Mode, Show};
use crate::sound;
use crate::{CHALLENGES_NAME, ID, TITLE_NAME};

/// The record, short, for a headline that already says what was played.
fn short_record(result: ScoreResult) -> String {
    match result {
        ScoreResult::First => "First score saved.".to_string(),
        ScoreResult::NewBest { previous } => format!("New best, up from {previous}."),
        ScoreResult::Short { best } => format!("Your best is {best}."),
    }
}

/// "Firework 1 scored 100, firework 2 scored 80." Each firework takes its
/// best match among the launches, so launches too few or too many are
/// named with what they cost.
fn per_firework(j: &Judgement) -> String {
    let each = j
        .per_target
        .iter()
        .enumerate()
        .map(|(i, p)| format!("firework {} scored {p}", i + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let mut line = format!("{}.", sentence_start(&each));
    let off = (j.launched as i64 - j.expected as i64).unsigned_abs();
    if off > 0 {
        let word = if j.launched < j.expected {
            "too few"
        } else {
            "too many"
        };
        line.push_str(&format!(" {off} {word}, {} off.", off * 20));
    }
    line
}

/// "Level 2 cleared, 78 percent, 780 points." or "Level 2 not cleared,
/// 35 percent, 60 needed."
fn headline(outcome: &str, j: &Judgement) -> String {
    if j.passed() {
        format!("{outcome}, {} percent, {} points.", j.accuracy, j.points)
    } else {
        format!("{outcome}, {} percent, {PASS_PERCENT} needed.", j.accuracy)
    }
}

/// The second page: the crowd, the launches and combos, after the points
/// of a show that did not clear when `points` (they still count there).
fn more(j: &Judgement, points: bool) -> String {
    let mut more = String::new();
    if points && !j.passed() {
        more.push_str(&format!("{} show points. ", j.points));
    }
    more.push_str(&format!(
        "{} You launched {} of {}.",
        j.crowd().line(),
        j.launched,
        j.expected
    ));
    if j.best_combo >= 2 {
        if j.combo_bonus > 0 {
            more.push_str(&format!(
                " Best combo {}, {} bonus points.",
                j.best_combo, j.combo_bonus
            ));
        } else {
            more.push_str(&format!(
                " Best combo {}, but combos only pay on a cleared show.",
                j.best_combo
            ));
        }
    }
    more
}

/// Compare: the show you were asked for, then yours.
fn compare(results: Results, show: &[Launch], launches: &[Launch]) -> Results {
    let (show, launches) = (show.to_vec(), launches.to_vec());
    results.extra(
        "Compare",
        "Hear the show you were asked for, then the one you launched, each after a count-in.",
        move |ctx| ctx.push(Replay::compare(show.clone(), launches.clone())),
    )
}

/// Scores the round: saves bests and progress, plays the crowd and a
/// stinger, and builds the results menu.
pub fn score(
    mode: Mode,
    show: Vec<Launch>,
    launches: Vec<Launch>,
    judgement: Judgement,
    ctx: &mut Ctx<Parlor>,
) -> Results {
    if let Mode::Challenge(run) = mode {
        return challenge(*run, show, launches, judgement, ctx);
    }
    if let Mode::Independence(run) = mode {
        return night(*run, show, launches, judgement, ctx);
    }
    let j = &judgement;
    let passed = j.passed();
    let total = mode.total() + if passed { j.points } else { 0 };
    let mut progress = Progress::load(ctx);
    let mut grand = false;
    // The headline: outcome, score, points, record. The rest goes on the
    // second page, which "Hear the results again" reads.
    let mut note = String::new();
    let outcome = match &mode {
        Mode::Level { level, .. } => {
            if passed && *level < TOTAL_LEVELS {
                format!("Level {level} cleared")
            } else if passed {
                grand = true;
                if !progress.finale_open() {
                    note = " The grand finale is open on the title.".to_string();
                }
                format!("All {TOTAL_LEVELS} levels cleared")
            } else {
                format!("Level {level} not cleared")
            }
        }
        Mode::Finale { round, .. } => {
            if passed {
                format!("Round {round} cleared")
            } else {
                let rounds = round - 1;
                let noun = if rounds == 1 { "round" } else { "rounds" };
                format!("The finale ends after {rounds} {noun} cleared")
            }
        }
        Mode::MyShow => if passed {
            "Your show matched"
        } else {
            "Your show not matched"
        }
        .to_string(),
        Mode::Challenge(_) | Mode::Independence(_) => String::new(),
    };
    let mut summary = if passed || !matches!(mode, Mode::Finale { .. }) {
        headline(&outcome, j)
    } else {
        format!("{outcome}, {} percent.", j.accuracy)
    };
    summary.push_str(&note);
    let mut more = more(j, !matches!(mode, Mode::Finale { .. }));
    if j.hint_used {
        more.push_str(" Hint used.");
    }

    match &mode {
        Mode::Level { level, .. } => {
            let r = Records::record(ctx, ID, &progress::level_board(*level), j.points);
            summary.push_str(&format!(" {}", short_record(r)));
            if passed {
                progress.clear(*level);
                // Replaying an earlier level never sends Start level back.
                progress.next_level = progress.next_level().max((*level + 1).min(TOTAL_LEVELS));
                more.push_str(&format!(" Run total {total}."));
                Records::record(ctx, ID, progress::TOTAL_BOARD, total);
            }
        }
        Mode::Finale { round, .. } => {
            let r = Records::record(ctx, ID, progress::FINALE_BOARD, total);
            // Each cleared round keeps the total, so a best equal to it
            // after round 1 is this finale's own.
            let own =
                !passed && *round > 1 && matches!(r, ScoreResult::Short { best } if best == total);
            if own {
                summary.push_str(&format!(" Finale total {total}, your best."));
            } else {
                summary.push_str(&format!(" Finale total {total}."));
                if !passed || matches!(r, ScoreResult::NewBest { .. }) {
                    summary.push_str(&format!(" {}", short_record(r)));
                }
            }
            grand = (own || matches!(r, ScoreResult::NewBest { .. })) && !passed;
        }
        Mode::MyShow => {
            let r = Records::record(ctx, ID, progress::MY_SHOW_BOARD, j.points);
            summary.push_str(&format!(" {}", short_record(r)));
        }
        Mode::Challenge(_) | Mode::Independence(_) => {}
    }
    if j.best_combo > progress.best_combo {
        progress.best_combo = j.best_combo;
    }
    progress.store(ctx);

    sound::react(ctx, j.accuracy);
    if j.combo_bonus > 0 {
        ctx.audio.play(sound::COMBO, Bus::UI);
    }
    let stinger = if grand {
        sound::FINALE
    } else if passed {
        sound::WIN
    } else {
        sound::LOSE
    };
    ctx.audio.play(stinger, Bus::MUSIC);

    let pages = vec![summary.clone(), more, per_firework(j)];
    let results = Results::new(summary, pages, Back::Title(TITLE_NAME));
    let results = match mode {
        Mode::Level { level, .. } => {
            // A replay starts from the run total before this level, so
            // replaying one level cannot pile up a run total.
            let carried = mode.total();
            let again = move |ctx: &mut Ctx<Parlor>| {
                ctx.replace(Show::new(Mode::Level {
                    level,
                    total: carried,
                }))
            };
            if passed && level < TOTAL_LEVELS {
                results
                    .next(
                        "Next level",
                        format!("Continue to level {}.", level + 1),
                        move |ctx| {
                            ctx.replace(Show::new(Mode::Level {
                                level: level + 1,
                                total,
                            }))
                        },
                    )
                    .again(
                        labels::PLAY_AGAIN,
                        format!("Play level {level} again with a new show."),
                        again,
                    )
            } else if passed {
                results.again(
                    labels::PLAY_AGAIN,
                    "Play the final level again with a new show.",
                    again,
                )
            } else {
                let same = show.clone();
                results
                    .again(
                        labels::TRY_AGAIN,
                        format!("Play level {level} again with a new show."),
                        again,
                    )
                    .extra(
                        "Practice this level",
                        "Play the same show again.",
                        move |ctx| {
                            ctx.replace(Show::with_show(
                                Mode::Level {
                                    level,
                                    total: carried,
                                },
                                same.clone(),
                            ))
                        },
                    )
            }
        }
        Mode::Finale { round, .. } => {
            if passed {
                results.next(
                    "Next round",
                    "A longer, faster show. The finale goes on until a round scores under 60.",
                    move |ctx| {
                        ctx.replace(Show::new(Mode::Finale {
                            round: round + 1,
                            total,
                        }))
                    },
                )
            } else {
                results.again(
                    labels::TRY_AGAIN,
                    "Start the grand finale over from round 1.",
                    |ctx| ctx.replace(Show::new(Mode::Finale { round: 1, total: 0 })),
                )
            }
        }
        Mode::MyShow => {
            let same = show.clone();
            results.again(
                if passed {
                    labels::PLAY_AGAIN
                } else {
                    labels::TRY_AGAIN
                },
                "Recreate your saved show once more.",
                move |ctx| ctx.replace(Show::with_show(Mode::MyShow, same.clone())),
            )
        }
        Mode::Challenge(_) | Mode::Independence(_) => results,
    };
    compare(results, &show, &launches)
}

/// Scores one show of a challenge run. When the run is over it is
/// submitted, and the result, best and streak are read out.
fn challenge(
    mut run: Run,
    show: Vec<Launch>,
    launches: Vec<Launch>,
    judgement: Judgement,
    ctx: &mut Ctx<Parlor>,
) -> Results {
    let j = &judgement;
    let passed = j.passed();
    let act = run.act().name.clone();
    let several = run.plan.acts.len() > 1;
    let over = run.record(j);
    let outcome = if several {
        format!("{act} {}", if passed { "cleared" } else { "not cleared" })
    } else if passed {
        "Show cleared".to_string()
    } else {
        "Show not cleared".to_string()
    };
    // The headline as levels have it; the rest on the second page.
    let mut summary = headline(&outcome, j);
    let mut more = more(j, true);
    if let Some(f) = run.plan.festival {
        if j.signature_bonus > 0 {
            more.push_str(&format!(
                " {} {} bonus points for your {}s.",
                hearken::text::sentence_start(f.name),
                j.signature_bonus,
                f.signature.name()
            ));
        }
    }
    if j.hint_used {
        more.push_str(" Hint used.");
    }
    let mut progress = Progress::load(ctx);
    if j.best_combo > progress.best_combo {
        progress.best_combo = j.best_combo;
        progress.store(ctx);
    }
    let ch = run.challenge.clone();
    if over {
        if several && run.plan.ends_on_fail && !passed && run.stage < run.plan.acts.len() {
            summary.push_str(&format!(
                " The gauntlet ends after {} of {} shows.",
                run.stage,
                run.plan.acts.len()
            ));
        }
        let result = Challenges::submit(ctx, &ch, run.total);
        let streak = Challenges::streak(ctx, ID, ch.period).max(1);
        let line = match result {
            ScoreResult::First => "Your first result on this challenge.".to_string(),
            ScoreResult::NewBest { previous } => {
                format!("A new best, up from {previous}.")
            }
            ScoreResult::Short { best } => format!("Your best is {best}."),
        };
        // One show's points are the result, already in the headline.
        if several {
            summary.push_str(&format!(
                " {} result: {} show points.",
                ch.title(),
                run.total
            ));
        }
        summary.push_str(&format!(
            " {line} {}",
            streak_sentence(streak, ch.period.unit())
        ));
        more.push_str(&format!(" {}", ch.resets()));
    } else {
        summary.push_str(&format!(" Challenge total {}.", run.total));
        more.push_str(&format!(
            " Next: {}, {}.",
            run.progress(),
            run.act().name.to_lowercase()
        ));
    }

    sound::react(ctx, j.accuracy);
    if j.combo_bonus > 0 || j.signature_bonus > 0 {
        ctx.audio.play(sound::COMBO, Bus::UI);
    }
    let stinger = if over && !matches!(Challenges::best(ctx, &ch), Some(b) if b > run.total) {
        sound::CHALLENGE
    } else if passed {
        sound::WIN
    } else {
        sound::LOSE
    };
    ctx.audio.play(stinger, Bus::MUSIC);

    let period = ch.period;
    let pages = vec![summary.clone(), more, per_firework(j)];
    let mut results = Results::new(summary, pages, Back::Challenges(CHALLENGES_NAME));
    if over {
        results = results.again(
            labels::challenge_again(period),
            "Start this challenge over from its briefing. Your best result is kept.",
            move |ctx| {
                let pages = crate::briefing(ctx, period);
                ctx.replace(pages)
            },
        );
    } else {
        let next = Box::new(run.clone());
        results = results.next(
            format!("Next: {}", run.act().name.to_lowercase()),
            format!(
                "Go on to {}. The challenge counts once its last show is over, so going back to the challenges now leaves it uncounted.",
                run.progress()
            ),
            move |ctx| ctx.replace(Show::new(Mode::Challenge(next.clone()))),
        );
    }
    compare(results, &show, &launches)
}

/// Scores one act of Independence night. After the last act the night is
/// kept: its best board, and this year's keepsake.
fn night(
    mut run: NightRun,
    show: Vec<Launch>,
    launches: Vec<Launch>,
    judgement: Judgement,
    ctx: &mut Ctx<Parlor>,
) -> Results {
    let j = &judgement;
    let passed = j.passed();
    let act = run.act().name.clone();
    let over = run.record(j);
    let outcome = format!("{act} {}", if passed { "cleared" } else { "not cleared" });
    // The headline as levels have it; the rest on the second page.
    let mut summary = headline(&outcome, j);
    let mut more = more(j, true);
    if j.signature_bonus > 0 {
        more.push_str(&format!(
            " The booms paid {} bonus points.",
            j.signature_bonus
        ));
    }
    if j.hint_used {
        more.push_str(" Hint used.");
    }
    let mut progress = Progress::load(ctx);
    if j.best_combo > progress.best_combo {
        progress.best_combo = j.best_combo;
        progress.store(ctx);
    }
    if over {
        Records::record(ctx, ID, independence::BOARD, run.total);
        let mut keep = Keepsakes::load(ctx);
        let kept = keep.keep(run.year, run.total, run.cleared);
        keep.store(ctx);
        let line = match kept {
            Kept::First => format!("Your keepsake for {} is saved.", run.year),
            Kept::Better { previous } => {
                format!("A new best for {}, up from {previous}.", run.year)
            }
            Kept::Short { best } => format!("Your best for {} is {best}.", run.year),
        };
        summary.push_str(&format!(
            " The night is over: {} show points, {} of {} acts cleared. {line}",
            run.total,
            run.cleared,
            run.acts.len()
        ));
        more.push_str(" Happy Fourth of July.");
    } else {
        summary.push_str(&format!(" Night total {}.", run.total));
        more.push_str(&format!(
            " Next: {}, {}.",
            run.progress(),
            run.act().name.to_lowercase()
        ));
    }

    sound::react(ctx, j.accuracy);
    if j.combo_bonus > 0 || j.signature_bonus > 0 {
        ctx.audio.play(sound::COMBO, Bus::UI);
    }
    let stinger = if over {
        sound::FINALE
    } else if passed {
        sound::WIN
    } else {
        sound::LOSE
    };
    ctx.audio.play(stinger, Bus::MUSIC);

    let pages = vec![summary.clone(), more, per_firework(j)];
    let mut results = Results::new(summary, pages, Back::Title(TITLE_NAME));
    if over {
        let year = run.year;
        results = results.again(
            labels::PLAY_AGAIN,
            "The whole night again, from the opening salute. Your best for the year is kept.",
            move |ctx| ctx.replace(Show::new(Mode::Independence(Box::new(NightRun::new(year))))),
        );
    } else {
        let next = Box::new(run.clone());
        results = results.next(
            format!("Next: {}", run.act().name.to_lowercase()),
            format!(
                "Go on to {}. The night is kept once its last act is over.",
                run.progress()
            ),
            move |ctx| ctx.replace(Show::new(Mode::Independence(next.clone()))),
        );
    }
    compare(results, &show, &launches)
}
