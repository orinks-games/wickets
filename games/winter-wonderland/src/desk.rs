//! The Wish Desk's rules: a stack of children's letters and a shelf of
//! wrapped boxes, with no clock.
//!
//! Each letter asks for one thing in a child's own words. The player walks
//! the shelf, shakes boxes to hear what is inside, and wraps the one that
//! answers the letter. A wrong box goes back on the shelf and costs the
//! letter some of its points; extra shakes cost a little. A stack ends when
//! every letter is answered, never on a clock.
//!
//! On Muddled post every box is packed in straw: its first shake is only
//! straw, and the second is clear. Late post brings two more letters at
//! the end of a stack, once an evening.

use parlor_core::Rng;

use crate::gifts::Kind;

/// The stacks in a full Wish Desk run.
pub const STACKS: u32 = 5;

/// Letters Late post adds to the end of a stack.
pub const LATE_LETTERS: u32 = 2;

/// Who writes in.
pub const CHILDREN: [&str; 10] = [
    "Milo", "Ada", "Rosie", "Sam", "Ivy", "Finn", "Lottie", "Arlo", "Noor", "Bea",
];

/// What a child calls each thing, in their own words.
pub fn wish_words(item: &str) -> &'static str {
    match item {
        "building blocks" => "blocks, so I can build the tallest tower ever",
        "a bag of dice" => "dice for playing games with Grandad",
        "a wooden puzzle" => "a wooden puzzle with lots and lots of pieces",
        "a teddy bear" => "a bear to hug at bedtime",
        "fluffy slippers" => "fluffy slippers, because the kitchen floor is cold",
        "a plush penguin" => "a soft penguin I can cuddle",
        "teacups" => "teacups for my dolls' tea party",
        "glass ornaments" => "shiny glass baubles to hang in the window",
        "a set of mugs" => "new mugs, because Dad broke ours",
        "bells" => "bells to wear when I dance",
        "a bell for the bike" => "a bell for my bike, so people hear me coming",
        "jingle bell earrings" => "earrings for Mum that jingle when she laughs",
        "chocolates in foil" => "chocolates in shiny paper",
        "a bag of toffees" => "toffees, the sticky kind that pull your teeth",
        "candy canes in cellophane" => "stripy sweets in see-through wrappers",
        "a snow globe" => "a ball with snow inside that swirls about",
        "bubble bath" => "bubbles for my bath, loads of them",
        "a bottle of maple syrup" => "syrup for Sunday pancakes",
        "a pocket watch" => "a watch for Grandpa's waistcoat pocket",
        "an alarm clock" => "a clock to wake me up for school, but not too early",
        "a kitchen timer" => "a timer so Dad stops burning the cakes",
        "a rubber duck" => "a duck to float in the bath",
        "a squeaky dog toy" => "a toy for my dog, Biscuit",
        "squeaky boots" => "new boots for splashing in puddles",
        "a stack of books" => "lots and lots of books",
        "a heavy cookbook" => "a big cooking book for Nana",
        "a bowling ball" => "a bowling ball, a really heavy one",
        "a music box" => "a box that plays a song when you open it",
        "a musical jewellery box" => "a box for Mum's rings that plays a tune",
        "a tiny carousel" => "the little horses that go round and sing",
        _ => "a surprise",
    }
}

/// How the youngest children ask for each kind, on Muddled post.
pub fn little_words(kind: Kind) -> &'static str {
    match kind {
        Kind::Rattle => "Bricks! Lots. For knocking down.",
        Kind::Squish => "A cuddly. Soft one. For bed.",
        Kind::Clink => "Cups. For pretend tea.",
        Kind::Jingle => "The ting ting one.",
        Kind::Crinkle => "Sweeties. Shiny ones.",
        Kind::Slosh => "The splashy one. Wet inside.",
        Kind::Tick => "Tick tock thing. Like Nana's.",
        Kind::Squeak => "Duck. Bath duck. Goes eek.",
        Kind::Thump => "Big heavy one. Boom.",
        Kind::Chime => "Song box. La la la.",
    }
}

/// How one stack plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StackSpec {
    pub level: u32,
    /// Letters in the stack, before any late post.
    pub letters: u32,
    /// Boxes on the shelf.
    pub boxes: usize,
    /// How many gift sounds can be on the shelf.
    pub pool: usize,
    /// Whether each letter is read with the sound its wish would make.
    pub say_sounds: bool,
    /// Every box is packed in straw: its first shake is only straw.
    pub straw: bool,
    /// The letters come from the youngest children, on Muddled post.
    pub little: bool,
    /// A wrong box costs this many of the letter's points.
    pub wrong_cost: u64,
    /// Shakes allowed on a letter before each one costs points.
    pub free_shakes: u32,
}

impl StackSpec {
    /// Stack `level`, 1 to [`STACKS`].
    pub fn new(level: u32) -> StackSpec {
        let l = level.clamp(1, STACKS);
        let i = (l - 1) as usize;
        let boxes = [4, 5, 6, 7, 8][i];
        StackSpec {
            level: l,
            letters: [3, 4, 4, 5, 6][i],
            boxes,
            pool: [4, 6, 8, 10, 10][i],
            say_sounds: l <= 2,
            straw: false,
            little: false,
            wrong_cost: 250,
            free_shakes: (boxes as u32).div_ceil(2),
        }
    }
}

/// One letter in the stack.
#[derive(Clone, Debug, PartialEq)]
pub struct Letter {
    pub from: &'static str,
    pub age: u32,
    pub wish: Kind,
    pub item: &'static str,
    /// From one of the youngest children, on Muddled post.
    pub little: bool,
    /// Came with the late post: double points.
    pub late: bool,
}

impl Letter {
    /// What the letter asks for, in the child's words.
    pub fn words(&self) -> String {
        if self.little {
            little_words(self.wish).to_string()
        } else {
            format!("Please may I have {}.", wish_words(self.item))
        }
    }
}

/// A box on the shelf.
#[derive(Clone, Debug, PartialEq)]
pub struct Parcel {
    pub kind: Kind,
    /// Shaken since the stack began.
    pub shakes: u32,
    /// Wrapped and sent: an empty place on the shelf now.
    pub gone: bool,
}

/// What a shake heard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shake {
    /// The gift inside.
    Gift(Kind),
    /// Only straw: the first shake of a box packed in straw.
    Straw,
    /// An empty place on the shelf.
    Empty,
}

/// What wrapping a box did.
#[derive(Clone, Debug, PartialEq)]
pub enum Wrapped {
    /// The right gift: the letter is answered.
    Right { points: u64, from: &'static str },
    /// Not what the letter asked for: back on the shelf.
    Wrong { from: &'static str, kind: Kind },
    /// Nothing there to wrap.
    Empty,
}

/// One stack of letters at the desk.
pub struct Stack {
    pub spec: StackSpec,
    pub letters: Vec<Letter>,
    pub shelf: Vec<Parcel>,
    /// The letter being answered, 0 based.
    pub current: usize,
    /// Where the player stands, 0 based.
    pub at: usize,
    pub score: u64,
    /// Shakes on the current letter.
    pub shakes: u32,
    /// Wrong boxes on the current letter.
    pub wrongs: u32,
    /// The current letter has had a hint: half points.
    pub hinted: bool,
    /// Every wrong box this stack.
    pub total_wrongs: u32,
    pub total_shakes: u32,
    pub hints: u32,
    rng: Rng,
}

impl Stack {
    pub fn new(spec: StackSpec, mut rng: Rng) -> Stack {
        let mut pool: Vec<Kind> = Kind::ALL[..spec.pool].to_vec();
        rng.shuffle(&mut pool);
        let wishes: Vec<Kind> = pool[..spec.letters as usize].to_vec();
        let mut kinds: Vec<Kind> = pool[..spec.boxes.min(pool.len())].to_vec();
        rng.shuffle(&mut kinds);
        let mut children = CHILDREN.to_vec();
        rng.shuffle(&mut children);
        let letters = wishes
            .iter()
            .enumerate()
            .map(|(i, &wish)| Letter {
                from: children[i % children.len()],
                age: 4 + rng.below(6),
                wish,
                item: rng.pick(wish.items()),
                little: false,
                late: false,
            })
            .collect();
        let shelf = kinds
            .into_iter()
            .map(|kind| Parcel {
                kind,
                shakes: 0,
                gone: false,
            })
            .collect();
        let mut stack = Stack {
            spec,
            letters,
            shelf,
            current: 0,
            at: 0,
            score: 0,
            shakes: 0,
            wrongs: 0,
            hinted: false,
            total_wrongs: 0,
            total_shakes: 0,
            hints: 0,
            rng,
        };
        if spec.little {
            stack.make_little();
        }
        stack
    }

    /// Muddled post: the letters are from the youngest children.
    fn make_little(&mut self) {
        for l in &mut self.letters {
            l.little = true;
            l.age = 3 + self.rng.below(2);
        }
    }

    /// The letter being answered, if any are left.
    pub fn letter(&self) -> Option<&Letter> {
        self.letters.get(self.current)
    }

    pub fn done(&self) -> bool {
        self.current >= self.letters.len()
    }

    /// Letters answered so far.
    pub fn answered(&self) -> usize {
        self.current.min(self.letters.len())
    }

    /// The box where the player stands.
    pub fn here(&self) -> &Parcel {
        &self.shelf[self.at]
    }

    /// Steps along the shelf. False at its end, where nothing moves.
    pub fn step(&mut self, by: i32) -> bool {
        let to = self.at as i32 + by;
        if to < 0 || to >= self.shelf.len() as i32 {
            return false;
        }
        self.at = to as usize;
        true
    }

    /// Shakes the box where the player stands.
    pub fn shake(&mut self) -> Shake {
        let straw = self.spec.straw;
        let p = &mut self.shelf[self.at];
        if p.gone {
            return Shake::Empty;
        }
        p.shakes += 1;
        self.shakes += 1;
        self.total_shakes += 1;
        if straw && p.shakes == 1 {
            Shake::Straw
        } else {
            Shake::Gift(p.kind)
        }
    }

    /// Where the current letter's gift is, 0 based.
    pub fn answer(&self) -> Option<usize> {
        let wish = self.letter()?.wish;
        self.shelf.iter().position(|p| !p.gone && p.kind == wish)
    }

    /// Takes a hint on the current letter: half its points.
    pub fn take_hint(&mut self) {
        if !self.hinted && !self.done() {
            self.hinted = true;
            self.hints += 1;
        }
    }

    /// The points the current letter would earn now.
    pub fn points_now(&self) -> u64 {
        let Some(letter) = self.letter() else {
            return 0;
        };
        let extra = self.shakes.saturating_sub(self.spec.free_shakes) as u64;
        let mut points = 750u64
            .saturating_sub(50 * extra)
            .saturating_sub(self.spec.wrong_cost * self.wrongs as u64)
            .max(150);
        if self.hinted {
            points /= 2;
        }
        if letter.late {
            points *= 2;
        }
        points * self.spec.level as u64
    }

    /// Wraps the box where the player stands for the current letter.
    pub fn wrap(&mut self) -> Wrapped {
        let Some(letter) = self.letter().cloned() else {
            return Wrapped::Empty;
        };
        if self.shelf[self.at].gone {
            return Wrapped::Empty;
        }
        let kind = self.shelf[self.at].kind;
        if kind != letter.wish {
            self.wrongs += 1;
            self.total_wrongs += 1;
            return Wrapped::Wrong {
                from: letter.from,
                kind,
            };
        }
        let points = self.points_now();
        self.score += points;
        self.shelf[self.at].gone = true;
        self.current += 1;
        self.shakes = 0;
        self.wrongs = 0;
        self.hinted = false;
        Wrapped::Right {
            points,
            from: letter.from,
        }
    }

    /// The bonus for a stack with no wrong box.
    pub fn neat_bonus(&self) -> u64 {
        if self.total_wrongs == 0 {
            500 * self.spec.level as u64
        } else {
            0
        }
    }

    /// Late post: two more letters, and a box for each on the shelf where
    /// earlier gifts went. True when it came.
    pub fn late_post(&mut self) -> bool {
        let free: Vec<usize> = (0..self.shelf.len())
            .filter(|&i| self.shelf[i].gone)
            .collect();
        if free.len() < LATE_LETTERS as usize {
            return false;
        }
        let on_shelf: Vec<Kind> = self
            .shelf
            .iter()
            .filter(|p| !p.gone)
            .map(|p| p.kind)
            .collect();
        let mut fresh: Vec<Kind> = Kind::ALL
            .iter()
            .copied()
            .filter(|k| !on_shelf.contains(k))
            .collect();
        self.rng.shuffle(&mut fresh);
        let mut children = CHILDREN.to_vec();
        self.rng.shuffle(&mut children);
        for (n, &slot) in free.iter().take(LATE_LETTERS as usize).enumerate() {
            let wish = fresh[n];
            self.shelf[slot] = Parcel {
                kind: wish,
                shakes: 0,
                gone: false,
            };
            let little = self.spec.little;
            self.letters.push(Letter {
                from: children[n],
                age: if little { 3 } else { 5 + self.rng.below(5) },
                wish,
                item: self.rng.pick(wish.items()),
                little,
                late: true,
            });
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(level: u32, seed: u64) -> Stack {
        Stack::new(StackSpec::new(level), Rng::seeded(seed))
    }

    #[test]
    fn every_letter_has_its_gift_on_the_shelf_once() {
        for level in 1..=STACKS {
            for seed in 0..40 {
                let s = stack(level, seed);
                assert_eq!(s.shelf.len(), s.spec.boxes);
                for l in &s.letters {
                    let n = s.shelf.iter().filter(|p| p.kind == l.wish).count();
                    assert_eq!(n, 1, "level {level} seed {seed}");
                }
            }
        }
    }

    #[test]
    fn wrapping_the_right_box_answers_the_letter() {
        let mut s = stack(1, 3);
        let at = s.answer().unwrap();
        s.at = at;
        assert_eq!(s.shake(), Shake::Gift(s.letters[0].wish));
        match s.wrap() {
            Wrapped::Right { points, .. } => assert_eq!(points, 750),
            other => panic!("{other:?}"),
        }
        assert_eq!(s.current, 1);
        assert_eq!(s.shake(), Shake::Empty);
        assert_eq!(s.wrap(), Wrapped::Empty);
    }

    #[test]
    fn a_wrong_box_and_extra_shakes_cost_points() {
        let mut s = stack(2, 5);
        let right = s.answer().unwrap();
        let wrong = (0..s.shelf.len()).find(|&i| i != right).unwrap();
        s.at = wrong;
        assert!(matches!(s.wrap(), Wrapped::Wrong { .. }));
        for _ in 0..s.spec.free_shakes + 2 {
            s.shake();
        }
        s.at = right;
        // 750, less 250 for the wrong box and 100 for two extra shakes, times 2.
        assert_eq!(s.points_now(), 800);
        s.take_hint();
        assert_eq!(s.points_now(), 400);
    }

    #[test]
    fn straw_needs_two_shakes() {
        let mut spec = StackSpec::new(3);
        spec.straw = true;
        let s = Stack::new(spec, Rng::seeded(9));
        assert!(s.letters.iter().all(|l| !l.little));
        spec.little = true;
        let mut s = Stack::new(spec, Rng::seeded(9));
        assert!(s.letters.iter().all(|l| l.little));
        assert_eq!(s.shake(), Shake::Straw);
        assert!(matches!(s.shake(), Shake::Gift(_)));
    }

    #[test]
    fn late_post_fills_empty_places_with_new_wishes() {
        let mut s = stack(1, 11);
        assert!(!s.late_post());
        while !s.done() {
            s.at = s.answer().unwrap();
            s.wrap();
        }
        assert!(s.late_post());
        assert!(!s.done());
        assert_eq!(s.letters.len(), 5);
        for l in &s.letters[3..] {
            assert!(l.late);
            let n = s
                .shelf
                .iter()
                .filter(|p| !p.gone && p.kind == l.wish)
                .count();
            assert_eq!(n, 1);
        }
        s.at = s.answer().unwrap();
        assert!(matches!(s.wrap(), Wrapped::Right { points: 1500, .. }));
    }

    #[test]
    fn every_item_has_a_childs_words() {
        for kind in Kind::ALL {
            for item in kind.items() {
                assert_ne!(wish_words(item), "a surprise", "{item}");
            }
        }
    }

    #[test]
    fn letters_read_in_the_childs_words() {
        let s = stack(4, 2);
        for l in &s.letters {
            let w = l.words();
            assert!(w.starts_with("Please may I have "), "{w}");
            assert!(!w.contains("a surprise"), "{w}");
        }
    }
}
