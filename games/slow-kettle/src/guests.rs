//! Who comes in: the regulars and their stories, the walk-ins, and the
//! words a guest says about their cup.

use crate::teas::Taste;

/// A regular guest, who comes back with the next part of their story.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Regular {
    /// A stable id, saved with how far their story has got.
    pub id: &'static str,
    pub name: &'static str,
    /// Who they are, said when they first come in: "a retired postman".
    pub who: &'static str,
    pub taste: Taste,
    /// Afternoons played before they first come in.
    pub joins_after: u32,
    /// Their story, one part a visit.
    pub story: [&'static str; 6],
    /// What they say once their story is told.
    pub after: [&'static str; 3],
    /// What they say at the midnight pot.
    pub midnight: &'static str,
}

pub const REGULARS: [Regular; 6] = [
    Regular {
        id: "hale",
        name: "Mister Hale",
        who: "a retired postman with a walking stick",
        taste: Taste::Strong,
        joins_after: 0,
        story: [
            "I walked my old round today, just for the walk. Number 9 still has the dog that used to chase me. He has gone grey, and so have I.",
            "The lady at number 14 kept every postcard her son ever sent. She read me three of them. He writes from the sea.",
            "A woman at the end of Elm Row asked me to carry a letter one more time, by hand, to her sister two streets over. They have not spoken in years.",
            "The sisters had tea together on Sunday. I was invited. I took biscuits, and nobody mentioned the years at all.",
            "The post office asked me to show the new carrier the round. She is quick. She already knows every dog by name.",
            "I have finished the whole round. Every door said hello. I think I will start again in spring, just for the walk.",
        ],
        after: [
            "Number 9's dog walked half the round with me today.",
            "The sisters send their regards. They are knitting matching scarves.",
            "My knees and I are getting along better these days.",
        ],
        midnight: "To old rounds and new ones.",
    },
    Regular {
        id: "june",
        name: "June",
        who: "a night-shift nurse, still in her coat",
        taste: Taste::Calm,
        joins_after: 0,
        story: [
            "Long shift. On the way home I bought a tomato plant, for no reason at all.",
            "The tomato plant has flowers. I talk to it when I get home at dawn. It is a very good listener.",
            "A pigeon has been eyeing my tomato plant. I have named him Gerald. We have come to an agreement.",
            "First tomato. Just one. I ate it on the balcony in my coat at six in the morning, and it was perfect.",
            "My neighbour saw the plant and asked for seeds. Now there are tomatoes on three balconies.",
            "The neighbours from all three balconies had a tomato party on the stairs. Gerald came too. I am on day shifts next month, but I will still come here.",
        ],
        after: [
            "Day shifts suit me. I see the sun now, and so does the tomato.",
            "Gerald brought a friend. The agreement is under review.",
            "I slept a whole night through. I had forgotten how good that is.",
        ],
        midnight: "To quiet nights, and full ones.",
    },
    Regular {
        id: "ada",
        name: "Ada",
        who: "a cello student, with her dog Biscuit at her heels",
        taste: Taste::Bright,
        joins_after: 1,
        story: [
            "Biscuit and I practised scales for an hour. He howled on every high note. I think he was being supportive.",
            "My teacher says I play too carefully. She says to play it like I am telling a friend a secret.",
            "I played in the park for the first time. Four people stopped. One of them was a toddler, who danced.",
            "I have a recital in the spring hall next month. I am terrified. Biscuit is not allowed in.",
            "The recital went well. I missed one note and nobody noticed but me. My teacher cried a little, the good kind.",
            "Biscuit has a bow tie for the next recital. They say he can sit at the back if he is quiet. He will not be quiet.",
        ],
        after: [
            "I am learning a piece that sounds like rain. Biscuit hides under the bed for it.",
            "The toddler from the park came back with her whole family.",
            "Biscuit sat through a whole rehearsal. One small howl. Progress.",
        ],
        midnight: "To the next recital, and Biscuit's bow tie.",
    },
    Regular {
        id: "wren",
        name: "Old Wren",
        who: "the town's clockmaker, with brass dust on his cuffs",
        taste: Taste::Sweet,
        joins_after: 3,
        story: [
            "The town hall clock has stood at ten past four for eleven years. The council has asked me to look at it.",
            "Up the tower today. Pigeons, dust, and one beautiful brass wheel with a cracked tooth.",
            "I am cutting a new tooth for the wheel by hand. Slow work, like your tea. You cannot hurry either.",
            "The clock ticks again. It loses two minutes a day, but it ticks. People keep stopping in the square to look up.",
            "Two minutes a day is now two seconds a week. I am very proud of those two seconds.",
            "It struck noon today, for the first time in eleven years. The whole square went quiet to listen. Then everyone clapped for a clock.",
        ],
        after: [
            "The clock is right to the second today. I checked it four times.",
            "A child asked me how the clock knows what time it is. I am still thinking about the answer.",
            "I have an apprentice now. She has better eyes than me, and steadier hands.",
        ],
        midnight: "To the clock in the square, and every second of the year.",
    },
    Regular {
        id: "tomas",
        name: "Tomas",
        who: "a bookbinder with glue on his thumbs",
        taste: Taste::Warming,
        joins_after: 5,
        story: [
            "A woman brought me a recipe book that is falling apart. It was her grandmother's. Every page smells of cinnamon.",
            "Inside the spine I found a folded note. A recipe for plum cake, with a little heart drawn beside it.",
            "The handwriting on the note is not the grandmother's. Her granddaughter has no idea whose it is.",
            "We found out. The note was from her grandfather, before they married. He could not cook at all. The plum cake was a proposal.",
            "The book is bound again, in green cloth. I sewed the note back inside the spine, right where it was.",
            "She baked the plum cake for her family and brought me a slice. It was a little burnt. She says that is the tradition now.",
        ],
        after: [
            "Someone brought me a diary from a lighthouse. I am taking my time with it.",
            "I bound a book of pressed flowers today. The whole shop smells of summer.",
            "The plum cake family sent me a card. They say it is still burnt, and still perfect.",
        ],
        midnight: "To old books, and the notes hidden in them.",
    },
    Regular {
        id: "priya",
        name: "Priya",
        who: "a student with a bag full of index cards",
        taste: Taste::Fresh,
        joins_after: 7,
        story: [
            "My exams are in six weeks. I am learning animal bones from little cards. There are a great many bones.",
            "I tested myself on the bus. The man next to me knew every bone of a horse. He used to shoe horses for a living.",
            "He invited me to his yard to meet his old pony, Pepper. Pepper ate my cards.",
            "Exams are over. I do not want to talk about it. I just want to sit here and drink this.",
            "I passed. I really passed. I told Pepper first.",
            "I start at the animal clinic on the hill next month. You will still see me, but I will smell of dogs.",
        ],
        after: [
            "A cat at the clinic sits on my notes the way Pepper ate them. I think it is a family of critics.",
            "Pepper had a check-up today. I did it myself. He was very patient.",
            "I have new cards now, for medicines. Fewer bones, more spelling.",
        ],
        midnight: "To Pepper, and to every patient animal.",
    },
];

pub fn regular_by_id(id: &str) -> Option<&'static Regular> {
    REGULARS.iter().find(|r| r.id == id)
}

/// The regulars who come in after `sessions` afternoons and evenings.
pub fn regulars_met(sessions: u32) -> Vec<&'static Regular> {
    REGULARS
        .iter()
        .filter(|r| r.joins_after <= sessions)
        .collect()
}

/// A walk-in guest: who they are and what they ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalkIn {
    /// Said as they come in: "A walker with muddy boots".
    pub who: &'static str,
    /// Their name in what they say: "The walker".
    pub short: &'static str,
    pub taste: Taste,
    pub asks: &'static str,
}

pub const WALK_INS: [WalkIn; 16] = [
    WalkIn {
        who: "A walker with muddy boots comes in.",
        short: "The walker",
        taste: Taste::Warming,
        asks: "Something to warm my hands, please.",
    },
    WalkIn {
        who: "A man in a rain-soaked coat comes in.",
        short: "The man",
        taste: Taste::Strong,
        asks: "Something strong. It has been a long day.",
    },
    WalkIn {
        who: "A woman with a sketchbook comes in.",
        short: "The artist",
        taste: Taste::Floral,
        asks: "Something that smells like a garden, if you have it.",
    },
    WalkIn {
        who: "A night-bus driver at the end of her shift comes in.",
        short: "The driver",
        taste: Taste::Calm,
        asks: "A calm cup before bed, please.",
    },
    WalkIn {
        who: "A teenager with headphones round her neck comes in.",
        short: "The teenager",
        taste: Taste::Bright,
        asks: "Something bright to wake me up?",
    },
    WalkIn {
        who: "A fisherman smelling of the harbour comes in.",
        short: "The fisherman",
        taste: Taste::Smoky,
        asks: "Something smoky and strong.",
    },
    WalkIn {
        who: "A grandfather and a small boy come in, hand in hand.",
        short: "The grandfather",
        taste: Taste::Sweet,
        asks: "Something a little sweet, without sugar, for the boy to try.",
    },
    WalkIn {
        who: "A cyclist comes in, out of breath.",
        short: "The cyclist",
        taste: Taste::Fresh,
        asks: "Something fresh and green, please.",
    },
    WalkIn {
        who: "A librarian on her lunch break comes in.",
        short: "The librarian",
        taste: Taste::Calm,
        asks: "Something gentle. The library was loud today, even for a library.",
    },
    WalkIn {
        who: "A builder with plaster on his hands comes in.",
        short: "The builder",
        taste: Taste::Strong,
        asks: "Something strong, please. As strong as you make it.",
    },
    WalkIn {
        who: "A painter with blue on her fingers comes in.",
        short: "The painter",
        taste: Taste::Fresh,
        asks: "Something clean and fresh, to clear my head.",
    },
    WalkIn {
        who: "A traveller with a heavy rucksack comes in.",
        short: "The traveller",
        taste: Taste::Warming,
        asks: "Something cosy, please. I have walked since dawn.",
    },
    WalkIn {
        who: "A florist with petals in her hair comes in.",
        short: "The florist",
        taste: Taste::Floral,
        asks: "Something flowery, please. I spend all day smelling roses and I never tire of it.",
    },
    WalkIn {
        who: "A chimney sweep with soot on his cheek comes in.",
        short: "The sweep",
        taste: Taste::Smoky,
        asks: "Something smoky. I know, I know. I like what I like.",
    },
    WalkIn {
        who: "A young dad with a sleeping baby in a sling comes in.",
        short: "The dad",
        taste: Taste::Bright,
        asks: "Something lively, please, quietly. She has only just gone off.",
    },
    WalkIn {
        who: "A beekeeper in a wide straw hat comes in.",
        short: "The beekeeper",
        taste: Taste::Sweet,
        asks: "Something soft and sweet, please. I have honey enough at home.",
    },
];

/// A member of the monthly tasting society, who asks for a tea by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Taster {
    pub who: &'static str,
    pub short: &'static str,
}

pub const TASTERS: [Taster; 5] = [
    Taster {
        who: "The society's president, in a velvet waistcoat, takes the first seat.",
        short: "The president",
    },
    Taster {
        who: "A taster with a silver spoon on a chain sits down.",
        short: "The taster",
    },
    Taster {
        who: "A quiet man with a notebook sits down.",
        short: "The quiet man",
    },
    Taster {
        who: "A lady with very good posture sits down.",
        short: "The lady",
    },
    Taster {
        who: "The society's youngest member, nervous and smiling, sits down.",
        short: "The youngest member",
    },
];

/// How a cup went, from best to gentlest. None of them is a failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Grade {
    Gentle,
    Pleasant,
    Lovely,
    Perfect,
}

impl Grade {
    pub fn word(self) -> &'static str {
        match self {
            Grade::Perfect => "perfect",
            Grade::Lovely => "lovely",
            Grade::Pleasant => "pleasant",
            Grade::Gentle => "kind",
        }
    }

    /// What the guest says about the cup, after their name.
    pub fn lines(self) -> &'static [&'static str] {
        match self {
            Grade::Perfect => &[
                "Oh. That is exactly right.",
                "That is the best cup I have had in a long time.",
                "Perfect. I did not know I needed this so much.",
                "Just right. You took your time, and it shows.",
            ],
            Grade::Lovely => &[
                "That is lovely, thank you.",
                "Mm. Lovely. Just what I wanted.",
                "Lovely and warm. Thank you.",
            ],
            Grade::Pleasant => &[
                "That is nice. Warm and kind.",
                "A good cup. Thank you.",
                "That is pleasant. I feel better already.",
            ],
            Grade::Gentle => &[
                "Not quite what I pictured, but it was kind of you to make it.",
                "It is warm, and that is what matters today.",
                "A cup of something is always better than a cup of nothing. Thank you.",
            ],
        }
    }
}

/// The notes a perfect cup leaves on the saucer.
pub const NOTES: [&str; 10] = [
    "Thank you for taking your time.",
    "This place feels like a deep breath.",
    "Same again next week, please.",
    "You made my whole day better.",
    "My grandmother made tea like this.",
    "I came in tired and I am leaving happy.",
    "The kettle sang just for me, I think.",
    "Here is to slow afternoons.",
    "A small thank you, for a big cup.",
    "I will tell my friends about this place.",
];
