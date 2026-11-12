//! The tutorial: the first course, one saucepan of cranberry sauce, in the
//! real kitchen.

use hearken::text::sentence_start;
use hearken::Ctx;
use parlor_core::{Lesson, Live, Parlor, ParlorAction};

use crate::kitchen::{Kitchen, Mode, Phase};
use crate::rules::{course, Grade};
use crate::ID;

/// The kitchen the lesson cooks in: course 1's saucepan. The lesson ends
/// as the dish is done, before the meal is served and scored. The helper
/// never steps in, so sitting idle can't finish the lesson.
pub fn kitchen() -> Live<Kitchen> {
    Live::new(Kitchen::with_course(Mode::Course(1), course(1)).in_lesson())
}

/// Holds the meal's clock while `line` is read, about 0.4 seconds plus
/// 15 characters a second, so no call or "Hurry" talks over the step.
fn hold_for(kitchen: &Live<Kitchen>, line: &str) -> impl FnMut(&mut Ctx<Parlor>) {
    let kitchen = kitchen.clone();
    let secs = 0.4 + line.chars().count() as f64 / 15.0;
    move |_ctx| kitchen.with_mut(|k| k.hold = secs)
}

/// "Perfect." for a step's timing score.
fn grade_of(score: Option<&u32>) -> &'static str {
    Grade::for_score(score.copied().unwrap_or(0)).word()
}

/// The tutorial, naming the player's own controls.
pub fn lesson(ctx: &mut Ctx<Parlor>) -> Lesson {
    lesson_on(ctx, kitchen())
}

/// The tutorial on `kitchen`, which a test can watch.
pub fn lesson_on(ctx: &mut Ctx<Parlor>, kitchen: Live<Kitchen>) -> Lesson {
    let k = |a| ctx.control_name(a);
    let (l, r, p, h) = (
        k(ParlorAction::Left),
        k(ParlorAction::Right),
        k(ParlorAction::Primary),
        k(ParlorAction::Hint),
    );

    let begin = kitchen.step(
        format!("{} starts cooking. Start now.", sentence_start(&p)),
        |k: &Kitchen, _| (k.phase == Phase::Cooking).then(String::new),
    );

    let start_text = format!(
        "The recipe is called out as you cook, and the called station signals. {} and {r} move there; {p} starts the dish. Starting before its call is a fumble. Start the cranberries on the left burner.",
        sentence_start(&l)
    );
    let start_hold = hold_for(&kitchen, &start_text);
    let start = kitchen
        .step(start_text, |k: &Kitchen, _| {
            let d = &k.meal.dishes[0];
            d.started()
                .then(|| format!("On the heat. {}", grade_of(d.scores.last())))
        })
        .start(start_hold);

    let tend_text = format!(
        "When the saucepan's sound changes, press {p} to stir. Act on that change for a perfect. Stir twice, then take it off the heat."
    );
    let tend_hold = hold_for(&kitchen, &tend_text);
    let tend = kitchen
        .step(tend_text, |k: &Kitchen, _| {
            let d = &k.meal.dishes[0];
            d.done().then(|| {
                format!(
                    "{} Cranberry sauce, done, quality {}.",
                    grade_of(d.scores.last()),
                    d.quality()
                )
            })
        })
        .start(tend_hold);

    Lesson::new(
        ID,
        "Harvest Feast",
        "Thanksgiving dinner: you run the kitchen, up to five stations in an arc in front of you. Learn on one saucepan",
    )
    .step(begin)
    .step(start)
    .step(tend)
    .outro(format!(
        "Later courses use more stations: {l} and {r} move between them, and each has its own cooking sound. {} says what needs you most. Play starts course 1.",
        sentence_start(&h)
    ))
}
