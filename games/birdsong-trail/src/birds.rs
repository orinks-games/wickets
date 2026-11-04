//! The birds of the trails: who sings, where, in which seasons, and how.
//!
//! Every song is synthesized for the game from three notes, low, middle
//! and high, on one of a handful of voices. The words in the field guide
//! describe the real bird's song as a listener might, but the sounds are
//! not recordings.

use parlor_core::Season;

/// One of the three notes every song is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Note {
    Low,
    Mid,
    High,
}

impl Note {
    pub const ALL: [Note; 3] = [Note::Low, Note::Mid, Note::High];

    /// "low", "middle", "high".
    pub fn word(self) -> &'static str {
        match self {
            Note::Low => "low",
            Note::Mid => "middle",
            Note::High => "high",
        }
    }

    /// The part of a sound key: "low", "mid", "high".
    pub fn slug(self) -> &'static str {
        match self {
            Note::Low => "low",
            Note::Mid => "mid",
            Note::High => "high",
        }
    }

    /// The next note up, wrapping from high to low.
    pub fn next(self) -> Note {
        match self {
            Note::Low => Note::Mid,
            Note::Mid => Note::High,
            Note::High => Note::Low,
        }
    }

    fn from_char(c: char) -> Option<Note> {
        match c {
            'L' => Some(Note::Low),
            'M' => Some(Note::Mid),
            'H' => Some(Note::High),
            _ => None,
        }
    }
}

/// Reads a song written as letters: "LMH" is low, middle, high.
pub fn parse(song: &str) -> Vec<Note> {
    song.chars().filter_map(Note::from_char).collect()
}

/// "low, high, middle": a song in words, for the hint and the call-outs.
pub fn notes_words(notes: &[Note]) -> String {
    notes
        .iter()
        .map(|n| n.word())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The synthesized voice a bird sings with. Each has a low, a middle and
/// a high note, and a bird plays them at its own pitch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Voice {
    /// Clear, sliding whistles: robins, blackbirds, cardinals.
    Whistle,
    /// Breathy, fluting notes: thrushes and warblers.
    Flute,
    /// Fast trills and rattles: wrens, larks, swallows.
    Trill,
    /// Round piping notes: tits, nuthatches, sandpipers.
    Piping,
    /// Buzzy, reedy notes: marsh birds.
    Reed,
    /// Thin, tinkling notes: finches and small birds.
    Tinkle,
    /// Soft, low cooing: doves, the cuckoo, the bittern.
    Coo,
    /// Deep, hollow hoots: owls.
    Hoot,
    /// A drumroll on hollow wood: the woodpecker.
    Drum,
    /// A goose's honk, for migrating flocks.
    Honk,
    /// A crane's rolling bugle, for migrating flocks.
    Bugle,
    /// A swan's whooping call, for migrating flocks.
    Whoop,
}

impl Voice {
    pub const ALL: [Voice; 12] = [
        Voice::Whistle,
        Voice::Flute,
        Voice::Trill,
        Voice::Piping,
        Voice::Reed,
        Voice::Tinkle,
        Voice::Coo,
        Voice::Hoot,
        Voice::Drum,
        Voice::Honk,
        Voice::Bugle,
        Voice::Whoop,
    ];

    pub fn slug(self) -> &'static str {
        match self {
            Voice::Whistle => "whistle",
            Voice::Flute => "flute",
            Voice::Trill => "trill",
            Voice::Piping => "piping",
            Voice::Reed => "reed",
            Voice::Tinkle => "tinkle",
            Voice::Coo => "coo",
            Voice::Hoot => "hoot",
            Voice::Drum => "drum",
            Voice::Honk => "honk",
            Voice::Bugle => "bugle",
            Voice::Whoop => "whoop",
        }
    }

    /// Whether the voice is a single flock call, with no notes.
    pub fn is_call(self) -> bool {
        matches!(self, Voice::Honk | Voice::Bugle | Voice::Whoop)
    }

    /// What the voice sounds like, for the sound check.
    pub fn words(self) -> &'static str {
        match self {
            Voice::Whistle => "clear, sliding whistles",
            Voice::Flute => "soft, breathy fluting",
            Voice::Trill => "quick, rattling trills",
            Voice::Piping => "round, piping notes",
            Voice::Reed => "buzzy, reedy notes",
            Voice::Tinkle => "thin, tinkling notes",
            Voice::Coo => "soft, low cooing",
            Voice::Hoot => "deep, hollow hoots",
            Voice::Drum => "a drumroll on hollow wood",
            Voice::Honk => "a goose's honk",
            Voice::Bugle => "a crane's rolling bugle",
            Voice::Whoop => "a swan's whooping call",
        }
    }
}

/// The four trails, each its own habitat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Habitat {
    Woodland,
    Meadow,
    Marsh,
    Riverside,
}

/// Where a bird can be met.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// On the trails, in its seasons.
    Trail,
    /// Only in the migration flocks of the equinoxes.
    Flock,
}

/// A kind of bird.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Species {
    /// A stable id, as the field guide saves it.
    pub id: &'static str,
    /// The name as said mid-sentence: "song thrush", "Baltimore oriole".
    pub name: &'static str,
    pub kind: Kind,
    pub voice: Voice,
    /// How far the voice is pitched up or down for this bird.
    pub pitch: f32,
    /// The signature song, as letters: "LMH" is low, middle, high.
    pub song: &'static str,
    pub habitats: &'static [Habitat],
    pub seasons: &'static [Season],
    /// The real bird's song, in words a listener can match.
    pub sounds_like: &'static str,
    /// One gentle line about the bird.
    pub about: &'static str,
}

use Habitat::{Marsh, Meadow, Riverside, Woodland};
use Season::{Autumn, Spring, Summer, Winter};

const ALL_YEAR: &[Season] = &[Spring, Summer, Autumn, Winter];
const WARM: &[Season] = &[Spring, Summer];
const COLD: &[Season] = &[Autumn, Winter];
const NOT_WINTER: &[Season] = &[Spring, Summer, Autumn];

#[allow(clippy::too_many_arguments)]
const fn trail(
    id: &'static str,
    name: &'static str,
    voice: Voice,
    pitch: f32,
    song: &'static str,
    habitats: &'static [Habitat],
    seasons: &'static [Season],
    sounds_like: &'static str,
    about: &'static str,
) -> Species {
    Species {
        id,
        name,
        kind: Kind::Trail,
        voice,
        pitch,
        song,
        habitats,
        seasons,
        sounds_like,
        about,
    }
}

const fn flock(
    id: &'static str,
    name: &'static str,
    voice: Voice,
    pitch: f32,
    sounds_like: &'static str,
    about: &'static str,
) -> Species {
    Species {
        id,
        name,
        kind: Kind::Flock,
        voice,
        pitch,
        song: "M",
        habitats: &[],
        seasons: &[Spring, Autumn],
        sounds_like,
        about,
    }
}

/// Every bird in the game, in field guide order.
pub const ALL: &[Species] = &[
    // The woodland.
    trail(
        "robin",
        "robin",
        Voice::Whistle,
        1.0,
        "MHLM",
        &[Woodland, Meadow],
        ALL_YEAR,
        "a run of clear, wistful whistles that tumble and rise",
        "A robin is often the first to sing in the morning and the last at dusk.",
    ),
    trail(
        "wren",
        "wren",
        Voice::Trill,
        1.15,
        "HHMHL",
        &[Woodland, Marsh],
        ALL_YEAR,
        "a loud, rapid rattle of bright notes, far too big for so small a bird",
        "The wren is one of the smallest birds on the trail, and one of the loudest.",
    ),
    trail(
        "blackbird",
        "blackbird",
        Voice::Whistle,
        0.85,
        "LMHM",
        &[Woodland, Meadow],
        ALL_YEAR,
        "a slow, mellow fluting, unhurried, like someone humming to themselves",
        "Blackbirds like a high perch for singing as the light comes up.",
    ),
    trail(
        "song-thrush",
        "song thrush",
        Voice::Flute,
        1.0,
        "HHLL",
        &[Woodland],
        ALL_YEAR,
        "each phrase sung twice or three times over, as if to be sure you heard it",
        "A song thrush cracks snail shells open on a favourite stone.",
    ),
    trail(
        "chickadee",
        "chickadee",
        Voice::Tinkle,
        1.0,
        "HM",
        &[Woodland],
        ALL_YEAR,
        "two sweet, clear notes, the second a little lower: fee, bee",
        "Chickadees stay all winter, and tuck seeds away to find later.",
    ),
    trail(
        "cardinal",
        "cardinal",
        Voice::Whistle,
        1.1,
        "HLLL",
        &[Woodland, Riverside],
        ALL_YEAR,
        "bright, sliding whistles, repeated: cheer, cheer, cheer",
        "Cardinals sing all year round, and both partners of a pair sing.",
    ),
    trail(
        "great-tit",
        "great tit",
        Voice::Piping,
        1.0,
        "HMHM",
        &[Woodland],
        ALL_YEAR,
        "a ringing two-note seesaw: teacher, teacher, teacher",
        "Great tits have dozens of different calls, and each bird knows several songs.",
    ),
    trail(
        "nuthatch",
        "nuthatch",
        Voice::Piping,
        1.2,
        "MMMM",
        &[Woodland],
        ALL_YEAR,
        "a quick, even run of piping notes, like a tiny whistle blown again and again",
        "A nuthatch can walk down a tree trunk head first.",
    ),
    trail(
        "woodpecker",
        "great spotted woodpecker",
        Voice::Drum,
        1.0,
        "HMMLL",
        &[Woodland],
        ALL_YEAR,
        "a fast drumroll on a hollow branch, fading as it goes",
        "A woodpecker drums instead of singing, choosing the branch that rings best.",
    ),
    trail(
        "chiffchaff",
        "chiffchaff",
        Voice::Tinkle,
        0.9,
        "LHLH",
        &[Woodland, Riverside],
        WARM,
        "two notes swapped back and forth, saying its own name: chiff, chaff",
        "The chiffchaff is one of the first migrants back each spring.",
    ),
    trail(
        "blackcap",
        "blackcap",
        Voice::Flute,
        1.1,
        "MHMHL",
        &[Woodland],
        WARM,
        "a rich, hurried warble that ends in a bright flourish",
        "Blackcaps arrive in spring and fill the woods with song from deep in the leaves.",
    ),
    trail(
        "willow-warbler",
        "willow warbler",
        Voice::Flute,
        1.2,
        "HHMML",
        &[Woodland, Riverside],
        WARM,
        "a soft, sweet cascade that slides gently down the scale",
        "A willow warbler weighs about as much as two coins, yet flies to Africa and back each year.",
    ),
    trail(
        "wood-thrush",
        "wood thrush",
        Voice::Flute,
        0.85,
        "LMH",
        &[Woodland],
        WARM,
        "a flute-like ee-oh-lay, with a soft trill at the end",
        "A wood thrush can sing two notes at once, one from each side of its voice box.",
    ),
    trail(
        "cuckoo",
        "cuckoo",
        Voice::Coo,
        1.1,
        "HL",
        &[Woodland, Meadow],
        &[Spring],
        "two soft, hollow notes, high then low: cuck, oo",
        "The cuckoo's call is heard for only a few weeks each spring.",
    ),
    trail(
        "redwing",
        "redwing",
        Voice::Tinkle,
        1.15,
        "HHM",
        &[Woodland, Meadow],
        COLD,
        "a thin, high seep, then a short, scratchy run",
        "Redwings come from the far north for the winter, travelling in loose, chattering flocks.",
    ),
    trail(
        "junco",
        "dark-eyed junco",
        Voice::Trill,
        0.9,
        "LLLL",
        &[Woodland, Meadow],
        &[Winter],
        "a steady, musical trill, all on one note",
        "Juncos are often called snowbirds, because they arrive with the first snow.",
    ),
    trail(
        "waxwing",
        "waxwing",
        Voice::Tinkle,
        1.3,
        "HH",
        &[Woodland, Riverside],
        &[Winter],
        "a thin, high, ringing trill, sung by the whole flock together",
        "Waxwings wander in winter wherever the berries are, and strip a bush in an afternoon.",
    ),
    // The meadow.
    trail(
        "skylark",
        "skylark",
        Voice::Trill,
        1.25,
        "HMHMH",
        &[Meadow],
        NOT_WINTER,
        "a long, bubbling stream of song from high overhead, going on and on",
        "A skylark sings as it climbs, hanging in the air until it is only a voice in the sky.",
    ),
    trail(
        "meadowlark",
        "meadowlark",
        Voice::Whistle,
        1.0,
        "HLMH",
        &[Meadow],
        NOT_WINTER,
        "a few clear, flute-like whistles that end in a gentle tumble",
        "A meadowlark sings from a fence post with its bright chest to the sun.",
    ),
    trail(
        "goldfinch",
        "goldfinch",
        Voice::Tinkle,
        1.1,
        "MHHM",
        &[Meadow, Riverside],
        ALL_YEAR,
        "a light, twittering tinkle, like a handful of small bells",
        "Goldfinches love thistle seed, and gather in little chattering groups.",
    ),
    trail(
        "yellowhammer",
        "yellowhammer",
        Voice::Trill,
        1.0,
        "MMMMH",
        &[Meadow],
        ALL_YEAR,
        "a row of quick, short notes and one long one: a little bit of bread and no cheese",
        "A yellowhammer sings from the top of a hedge, all through the warm afternoons.",
    ),
    trail(
        "swallow",
        "barn swallow",
        Voice::Trill,
        1.1,
        "HMLMH",
        &[Meadow, Riverside],
        WARM,
        "a fast, cheerful twitter with a dry rattle in the middle",
        "Swallows arrive in spring, nest in barns, and leave for the south in autumn.",
    ),
    trail(
        "bobolink",
        "bobolink",
        Voice::Reed,
        1.2,
        "LMHHM",
        &[Meadow],
        WARM,
        "a bubbling, tumbling jumble of notes, like a tune played on a toy",
        "Bobolinks nest in tall grass and fly to South America for the winter.",
    ),
    trail(
        "indigo-bunting",
        "indigo bunting",
        Voice::Piping,
        1.1,
        "HHMMH",
        &[Meadow],
        WARM,
        "bright notes in pairs, each pair a little different from the last",
        "Indigo buntings migrate by night and steer by the stars.",
    ),
    trail(
        "song-sparrow",
        "song sparrow",
        Voice::Trill,
        0.95,
        "MMMHL",
        &[Meadow, Marsh],
        ALL_YEAR,
        "three or four clear opening notes, then a jumbled, cheerful trill",
        "A young song sparrow learns its tunes by listening to its neighbours.",
    ),
    trail(
        "mourning-dove",
        "mourning dove",
        Voice::Coo,
        0.9,
        "MHLL",
        &[Meadow, Woodland],
        ALL_YEAR,
        "a soft, sad cooing, low and slow",
        "Mourning doves coo so softly that people often take them for owls.",
    ),
    trail(
        "snow-bunting",
        "snow bunting",
        Voice::Tinkle,
        1.2,
        "MHMH",
        &[Meadow],
        &[Winter],
        "a soft, rippling twitter as the flock lifts off the field",
        "Snow buntings swirl over winter fields like a flurry of snowflakes.",
    ),
    trail(
        "fieldfare",
        "fieldfare",
        Voice::Reed,
        0.85,
        "LLM",
        &[Meadow, Woodland],
        COLD,
        "a harsh, chuckling chack, chack, as the flock passes over",
        "Fieldfares come south for the winter and roam the fields in noisy bands.",
    ),
    // The marsh.
    trail(
        "red-winged-blackbird",
        "red-winged blackbird",
        Voice::Reed,
        1.0,
        "LMHH",
        &[Marsh],
        WARM,
        "a gurgling conk-la-ree, ending in a buzzing trill",
        "Red-winged blackbirds claim a reed each spring and sing from the top of it.",
    ),
    trail(
        "reed-warbler",
        "reed warbler",
        Voice::Reed,
        1.15,
        "MLMLM",
        &[Marsh],
        WARM,
        "a steady, chattering churr, like a little engine among the reeds",
        "A reed warbler weaves its nest around three or four reed stems.",
    ),
    trail(
        "sedge-warbler",
        "sedge warbler",
        Voice::Reed,
        1.25,
        "HLMHL",
        &[Marsh],
        WARM,
        "a scratchy, hurried medley with bright whistles thrown in",
        "A sedge warbler never sings quite the same song twice.",
    ),
    trail(
        "marsh-wren",
        "marsh wren",
        Voice::Trill,
        1.05,
        "LHHH",
        &[Marsh],
        NOT_WINTER,
        "a gurgle, then a fast, rattling buzz, over and over",
        "A marsh wren builds several nests, and lets its mate choose one.",
    ),
    trail(
        "yellowthroat",
        "common yellowthroat",
        Voice::Piping,
        1.05,
        "HMLHM",
        &[Marsh],
        WARM,
        "a rolling, repeated witchety, witchety, witchety",
        "The yellowthroat wears a black mask, like a tiny bandit in the reeds.",
    ),
    trail(
        "reed-bunting",
        "reed bunting",
        Voice::Tinkle,
        0.9,
        "MMH",
        &[Marsh],
        ALL_YEAR,
        "a short, simple song of a few notes, stopping almost before it starts",
        "Reed buntings sit on a tall stem and sway with it in the wind.",
    ),
    trail(
        "bittern",
        "bittern",
        Voice::Coo,
        0.7,
        "LL",
        &[Marsh],
        &[Spring],
        "a deep, hollow boom, like someone blowing across a bottle",
        "A bittern's boom carries for miles, but the bird itself is almost never seen.",
    ),
    trail(
        "water-rail",
        "water rail",
        Voice::Reed,
        0.9,
        "HML",
        &[Marsh],
        COLD,
        "a sudden squeal and a few grunts, like a small piglet in the reeds",
        "Water rails slip between the reed stems, thin enough to pass where nothing else can.",
    ),
    // The riverside.
    trail(
        "kingfisher",
        "kingfisher",
        Voice::Tinkle,
        1.25,
        "HHHH",
        &[Riverside],
        ALL_YEAR,
        "a sharp, shrill whistle as it darts low over the water",
        "A kingfisher watches from a low branch, then dives straight in for a fish.",
    ),
    trail(
        "dipper",
        "dipper",
        Voice::Trill,
        1.0,
        "HMHLM",
        &[Riverside],
        ALL_YEAR,
        "a sweet, scratchy warble that carries over rushing water",
        "A dipper walks along the stream bed underwater, looking for insects.",
    ),
    trail(
        "grey-wagtail",
        "grey wagtail",
        Voice::Piping,
        1.25,
        "MH",
        &[Riverside],
        ALL_YEAR,
        "a sharp, metallic call, two notes quick together",
        "A grey wagtail bobs its long tail all the time, from rock to rock.",
    ),
    trail(
        "sand-martin",
        "sand martin",
        Voice::Trill,
        1.2,
        "MHM",
        &[Riverside],
        WARM,
        "a dry, buzzy twitter as it skims the river",
        "Sand martins dig their nests in the soft earth of the riverbank.",
    ),
    trail(
        "sandpiper",
        "common sandpiper",
        Voice::Piping,
        1.15,
        "HHHM",
        &[Riverside],
        WARM,
        "thin, high piping notes, three at a time, as it flies low",
        "A sandpiper flies with stiff, flickering wings, just above the water.",
    ),
    trail(
        "yellow-warbler",
        "yellow warbler",
        Voice::Flute,
        1.25,
        "HHHML",
        &[Riverside],
        WARM,
        "bright, quick notes that rush to the end: sweet, sweet, I'm so sweet",
        "Yellow warblers nest in the willows by the water, and sing from dawn to dusk.",
    ),
    trail(
        "oriole",
        "Baltimore oriole",
        Voice::Whistle,
        1.0,
        "MHML",
        &[Riverside, Woodland],
        WARM,
        "rich, clear whistles, like a short tune on a wooden flute",
        "An oriole weaves a hanging nest, like a little bag, at the end of a branch.",
    ),
    trail(
        "siskin",
        "siskin",
        Voice::Trill,
        1.25,
        "HMH",
        &[Riverside],
        COLD,
        "a twittering, wheezy song from the tops of the alders",
        "Siskins hang upside down on alder cones to pick out the seeds.",
    ),
    // The migrating flocks of the equinoxes.
    flock(
        "canada-goose",
        "Canada goose",
        Voice::Honk,
        1.0,
        "loud, cheerful honks, each bird answering the others",
        "Geese fly in a long V, taking turns at the front where the work is hardest.",
    ),
    flock(
        "sandhill-crane",
        "sandhill crane",
        Voice::Bugle,
        1.0,
        "a rolling, rattling bugle that carries for miles",
        "Sandhill cranes are one of the oldest kinds of bird still flying today.",
    ),
    flock(
        "whooper-swan",
        "whooper swan",
        Voice::Whoop,
        1.0,
        "soft, bugling whoops, like a brass band warming up",
        "Whooper swans call to keep the family together on the long flight.",
    ),
    flock(
        "pink-footed-goose",
        "pink-footed goose",
        Voice::Honk,
        1.25,
        "high, yapping honks, wink wink, from a great skein of birds",
        "Pink-footed geese travel in huge flocks, thousands of birds at once.",
    ),
];

impl Species {
    /// The bird by its id.
    pub fn by_id(id: &str) -> Option<&'static Species> {
        ALL.iter().find(|s| s.id == id)
    }

    /// Its place in [`ALL`].
    pub fn index(&self) -> usize {
        ALL.iter().position(|s| s.id == self.id).unwrap_or(0)
    }

    /// The signature song.
    pub fn notes(&self) -> Vec<Note> {
        parse(self.song)
    }

    /// How many songs it has to learn: 3 verses for a singing bird, and
    /// one count for a flock bird.
    pub fn verses(&self) -> u32 {
        match self.kind {
            Kind::Flock => 1,
            _ => VERSES,
        }
    }

    /// Verse `n` of its song: the signature first, then two variations.
    pub fn verse(&self, n: u32) -> Vec<Note> {
        verse(&self.notes(), n)
    }

    /// "a robin", "an eastern screech owl".
    pub fn a_name(&self) -> String {
        a(self.name)
    }

    pub fn sings_in(&self, season: Season) -> bool {
        self.seasons.contains(&season)
    }

    pub fn lives_in(&self, habitat: Habitat) -> bool {
        self.habitats.contains(&habitat)
    }
}

/// Songs a singing bird has to learn.
pub const VERSES: u32 = 3;

/// "a" or "an" before a name.
pub fn a(name: &str) -> String {
    let vowel = name
        .chars()
        .next()
        .is_some_and(|c| "aeiouAEIOU".contains(c));
    if vowel {
        format!("an {name}")
    } else {
        format!("a {name}")
    }
}

/// Verse `n` of a signature song: 0 is the song itself; 1 adds a note at
/// the end, or for a five-note song changes the last; 2 is the song
/// backwards, or with its first note changed when backwards is the same.
pub fn verse(song: &[Note], n: u32) -> Vec<Note> {
    let mut out = song.to_vec();
    match n {
        0 => {}
        1 => {
            if out.len() < 5 {
                let first = out[0];
                out.push(first.next());
            } else if let Some(last) = out.last_mut() {
                *last = last.next();
            }
        }
        _ => {
            out.reverse();
            if out == song || out == verse(song, 1) {
                out = song.to_vec();
                out[0] = out[0].next();
            }
        }
    }
    out
}

/// The singing birds of a habitat in a season.
pub fn on_trail(habitat: Habitat, season: Season) -> Vec<&'static Species> {
    ALL.iter()
        .filter(|s| s.kind == Kind::Trail && s.lives_in(habitat) && s.sings_in(season))
        .collect()
}

/// Every singing trail bird of a season, on any trail: the dawn chorus.
pub fn of_season(season: Season) -> Vec<&'static Species> {
    ALL.iter()
        .filter(|s| s.kind == Kind::Trail && s.sings_in(season))
        .collect()
}

/// The birds of the migration flocks.
pub fn flocks() -> Vec<&'static Species> {
    ALL.iter().filter(|s| s.kind == Kind::Flock).collect()
}

/// The migrants a season brings that the one before did not have, on a
/// habitat: "the chiffchaff is back" in spring.
pub fn arrivals(habitat: Habitat, season: Season) -> Vec<&'static Species> {
    let before = match season {
        Spring => Winter,
        Summer => Spring,
        Autumn => Summer,
        Winter => Autumn,
    };
    on_trail(habitat, season)
        .into_iter()
        .filter(|s| !s.sings_in(before))
        .collect()
}

/// The birds of a habitat that have left since the season before.
pub fn departures(habitat: Habitat, season: Season) -> Vec<&'static Species> {
    let before = match season {
        Spring => Winter,
        Summer => Spring,
        Autumn => Summer,
        Winter => Autumn,
    };
    on_trail(habitat, before)
        .into_iter()
        .filter(|s| !s.sings_in(season))
        .collect()
}
