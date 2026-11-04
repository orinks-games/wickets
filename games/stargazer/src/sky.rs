//! The night sky: every constellation, star and wonder the game knows, and
//! where each stands in the evening sky of each season.
//!
//! The sky is a grid of steps of 5 degrees. Azimuth runs clockwise from
//! north in [`AZ_STEPS`] steps (0 north, 18 east, 36 south, 54 west), and
//! altitude from the horizon (0) to straight overhead ([`ZENITH`]). Every
//! star sits on a step, so the arrows land on it exactly. The positions are
//! the evening sky of a northern hilltop at about 40 degrees north, drawn a
//! little larger and simpler than life so that each star has its own step.

use parlor_core::{Rng, Season};

/// Steps of azimuth round the sky: 72 steps of 5 degrees.
pub const AZ_STEPS: i32 = 72;
/// The altitude step straight overhead.
pub const ZENITH: i32 = 18;
/// Degrees in one step.
pub const STEP_DEGREES: i32 = 5;

/// One star of a constellation, placed by its offset in steps from the
/// constellation's centre: `dx` to the right (clockwise), `dy` up.
#[derive(Clone, Copy, Debug)]
pub struct StarDef {
    pub name: &'static str,
    /// Apparent magnitude: smaller is brighter.
    pub mag: f32,
    pub dx: i32,
    pub dy: i32,
}

/// A constellation: its stars in tracing order, where it stands in each
/// season it is up, and its lore.
#[derive(Clone, Copy, Debug)]
pub struct ConDef {
    /// The saved id and the motif's sound name.
    pub id: &'static str,
    pub name: &'static str,
    /// What it pictures: "the hunter".
    pub figure: &'static str,
    /// 0 is up from the first night; 1 and 2 rise as the journal grows.
    pub tier: u8,
    /// The centre for each season it is up in the evening.
    pub places: &'static [(Season, i32, i32)],
    pub stars: &'static [StarDef],
    pub lore: &'static str,
}

impl ConDef {
    /// The lore, without an opening sentence that only names the figure,
    /// for lines that have just said it: "The winged horse. Its Great
    /// Square..." becomes "Its Great Square...".
    pub fn lore_after_figure(&self) -> &'static str {
        let lore = self.lore;
        let opening = format!("{}. ", self.figure);
        match lore.get(..opening.len()) {
            Some(head) if head.eq_ignore_ascii_case(&opening) => &lore[opening.len()..],
            _ => lore,
        }
    }

    /// Up in every season, circling the pole star.
    pub fn circumpolar(&self) -> bool {
        self.places.len() == Season::ALL.len()
    }

    pub fn centre(&self, season: Season) -> Option<(i32, i32)> {
        self.places
            .iter()
            .find(|(s, _, _)| *s == season)
            .map(|&(_, az, alt)| (az, alt))
    }

    /// Where its star `i` stands in `season`.
    pub fn star_at(&self, season: Season, i: usize) -> Option<(i32, i32)> {
        let (az, alt) = self.centre(season)?;
        let s = self.stars.get(i)?;
        Some((wrap_az(az + s.dx), alt + s.dy))
    }
}

/// Something soft to find: a cluster, a nebula, a galaxy.
#[derive(Clone, Copy, Debug)]
pub struct WonderDef {
    pub id: &'static str,
    pub name: &'static str,
    pub line: &'static str,
    pub places: &'static [(Season, i32, i32)],
}

impl WonderDef {
    pub fn at(&self, season: Season) -> Option<(i32, i32)> {
        self.places
            .iter()
            .find(|(s, _, _)| *s == season)
            .map(|&(_, az, alt)| (az, alt))
    }
}

/// Azimuth wrapped into `0..AZ_STEPS`.
pub fn wrap_az(az: i32) -> i32 {
    az.rem_euclid(AZ_STEPS)
}

/// The shortest way round from `from` to `to`, in steps: positive is to
/// the right, clockwise.
pub fn az_delta(from: i32, to: i32) -> i32 {
    let d = wrap_az(to - from);
    if d > AZ_STEPS / 2 {
        d - AZ_STEPS
    } else {
        d
    }
}

/// The angle between two points of the sky, in degrees.
pub fn angle(a: (i32, i32), b: (i32, i32)) -> f64 {
    let rad = |steps: i32| (steps * STEP_DEGREES) as f64 * std::f64::consts::PI / 180.0;
    let (az1, alt1) = (rad(a.0), rad(a.1));
    let (az2, alt2) = (rad(b.0), rad(b.1));
    let c = alt1.sin() * alt2.sin() + alt1.cos() * alt2.cos() * (az1 - az2).cos();
    c.clamp(-1.0, 1.0).acos().to_degrees()
}

/// "north", "northeast" ... the nearest of eight directions.
pub fn compass(az: i32) -> &'static str {
    const NAMES: [&str; 8] = [
        "north",
        "northeast",
        "east",
        "southeast",
        "south",
        "southwest",
        "west",
        "northwest",
    ];
    let deg = wrap_az(az) * STEP_DEGREES;
    NAMES[(((deg as f64 + 22.5) / 45.0).floor() as usize) % 8]
}

const fn s(name: &'static str, mag: f32, dx: i32, dy: i32) -> StarDef {
    StarDef { name, mag, dx, dy }
}

use Season::{Autumn, Spring, Summer, Winter};

/// Every constellation, circumpolar ones first.
pub const CONSTELLATIONS: &[ConDef] = &[
    ConDef {
        id: "ursa_major",
        name: "Ursa Major",
        figure: "the great bear",
        tier: 0,
        places: &[(Winter, 7, 6), (Spring, 4, 14), (Summer, 64, 9), (Autumn, 68, 4)],
        stars: &[
            s("Dubhe", 1.8, 0, 0),
            s("Merak", 2.3, 0, -2),
            s("Phecda", 2.4, 2, -2),
            s("Megrez", 3.3, 2, 0),
            s("Alioth", 1.8, 3, 1),
            s("Mizar", 2.2, 4, 1),
            s("Alkaid", 1.9, 5, 0),
        ],
        lore: "The great bear. Its seven brightest stars make the Big Dipper, and the two at the front of the bowl, Merak and Dubhe, point the way to Polaris, the pole star.",
    },
    ConDef {
        id: "cassiopeia",
        name: "Cassiopeia",
        figure: "the seated queen",
        tier: 0,
        places: &[(Winter, 62, 11), (Spring, 66, 4), (Summer, 12, 7), (Autumn, 70, 13)],
        stars: &[
            s("Caph", 2.3, 0, 0),
            s("Schedar", 2.2, 1, -1),
            s("Navi", 2.4, 2, 0),
            s("Ruchbah", 2.7, 3, -1),
            s("Segin", 3.4, 4, 0),
        ],
        lore: "From Greek myth, a W of stars across the Milky Way. She circles the pole opposite the Big Dipper, so when one is low, the other rides high.",
    },
    ConDef {
        id: "ursa_minor",
        name: "Ursa Minor",
        figure: "the little bear",
        tier: 1,
        places: &[(Winter, 0, 8), (Spring, 0, 8), (Summer, 0, 8), (Autumn, 0, 8)],
        stars: &[
            s("Polaris", 2.0, 0, 0),
            s("Yildun", 4.4, 1, 1),
            s("Kochab", 2.1, 3, 2),
            s("Pherkad", 3.0, 2, 3),
        ],
        lore: "The little bear. Polaris, at the tip of its tail, sits almost exactly above the North Pole, so the whole sky seems to turn around it while it stays still.",
    },
    ConDef {
        id: "cepheus",
        name: "Cepheus",
        figure: "the king",
        tier: 2,
        places: &[(Winter, 66, 14), (Spring, 62, 8), (Summer, 8, 10), (Autumn, 64, 9)],
        stars: &[
            s("Alderamin", 2.5, 0, 0),
            s("Alfirk", 3.2, 1, 1),
            s("Errai", 3.2, 0, 2),
        ],
        lore: "Husband of Cassiopeia, drawn like a little house with a pointed roof. The slow wobble of the Earth will make Errai the pole star in about two thousand years.",
    },
    ConDef {
        id: "draco",
        name: "Draco",
        figure: "the dragon",
        tier: 2,
        places: &[(Winter, 70, 4), (Spring, 14, 8), (Summer, 6, 15), (Autumn, 58, 8)],
        stars: &[
            s("Eltanin", 2.2, 0, 0),
            s("Rastaban", 2.8, 1, 1),
            s("Edasich", 3.3, -2, -1),
            s("Thuban", 3.7, -4, -2),
        ],
        lore: "It winds between the two bears. Thuban, near its tail, was the pole star when the great pyramids of Egypt were built.",
    },
    // Winter.
    ConDef {
        id: "orion",
        name: "Orion",
        figure: "the hunter",
        tier: 0,
        places: &[(Winter, 36, 8)],
        stars: &[
            s("Betelgeuse", 0.5, -2, 3),
            s("Bellatrix", 1.6, 1, 3),
            s("Mintaka", 2.2, 1, 0),
            s("Alnilam", 1.7, 0, 0),
            s("Alnitak", 1.8, -1, 0),
            s("Saiph", 2.1, -2, -3),
            s("Rigel", 0.1, 2, -3),
        ],
        lore: "The brightest pattern of winter. Red Betelgeuse marks a shoulder and blue white Rigel a foot, and the three stars of the belt stand almost in a perfect line.",
    },
    ConDef {
        id: "taurus",
        name: "Taurus",
        figure: "the bull",
        tier: 1,
        places: &[(Winter, 42, 12)],
        stars: &[
            s("Aldebaran", 0.9, 0, 0),
            s("Ain", 3.5, 1, 1),
            s("Elnath", 1.7, -1, 4),
            s("Tianguan", 3.0, 1, 3),
        ],
        lore: "It charges across the sky. Orange Aldebaran is its fiery eye, and the little cluster of the Pleiades rides on its shoulder.",
    },
    ConDef {
        id: "gemini",
        name: "Gemini",
        figure: "the twins",
        tier: 1,
        places: &[(Winter, 24, 12)],
        stars: &[
            s("Castor", 1.6, 1, 3),
            s("Pollux", 1.1, -1, 3),
            s("Wasat", 3.5, -1, 0),
            s("Alhena", 1.9, -1, -3),
            s("Mebsuta", 3.0, 1, -1),
        ],
        lore: "Castor and Pollux, side by side. Pollux is the brighter and a little orange. Castor looks like one star but is really six, bound together by gravity.",
    },
    ConDef {
        id: "canis_major",
        name: "Canis Major",
        figure: "the great dog",
        tier: 2,
        places: &[(Winter, 30, 3)],
        stars: &[
            s("Sirius", -1.5, 0, 2),
            s("Mirzam", 2.0, 2, 2),
            s("Wezen", 1.8, -1, 0),
            s("Adhara", 1.5, 1, -1),
            s("Aludra", 2.4, -3, -1),
        ],
        lore: "It follows at Orion's heel. Its star Sirius is the brightest in the whole night sky, and one of the nearest, under 9 light years away.",
    },
    ConDef {
        id: "perseus",
        name: "Perseus",
        figure: "the hero",
        tier: 2,
        places: &[(Winter, 58, 15), (Autumn, 10, 8)],
        stars: &[
            s("Mirfak", 1.8, 0, 0),
            s("Algol", 2.1, 1, -2),
            s("Atik", 2.8, 2, -4),
            s("Miram", 3.8, -1, 1),
        ],
        lore: "He holds the head of Medusa, whose eye is the star Algol. It fades for a few hours every 2 days and 21 hours, as a dimmer partner passes in front of it.",
    },
    // Spring.
    ConDef {
        id: "leo",
        name: "Leo",
        figure: "the lion",
        tier: 0,
        places: &[(Spring, 36, 11)],
        stars: &[
            s("Regulus", 1.4, 1, -2),
            s("Algieba", 2.0, 1, 1),
            s("Zosma", 2.6, -3, 1),
            s("Denebola", 2.1, -5, 0),
            s("Chertan", 3.3, -3, -1),
        ],
        lore: "A pattern of spring. A backwards question mark of stars, called the Sickle, makes its head and mane, with Regulus at the bottom like a jewel on its chest.",
    },
    ConDef {
        id: "bootes",
        name: "Bootes",
        figure: "the herdsman",
        tier: 1,
        places: &[(Spring, 20, 10)],
        stars: &[
            s("Muphrid", 2.7, 1, -3),
            s("Arcturus", -0.05, 0, -2),
            s("Izar", 2.4, -1, 0),
            s("Nekkar", 3.5, -1, 4),
            s("Seginus", 3.0, 1, 3),
        ],
        lore: "Shaped like a kite. Follow the curve of the Big Dipper's handle and you arc to Arcturus, the brightest star of the northern spring.",
    },
    ConDef {
        id: "virgo",
        name: "Virgo",
        figure: "the maiden",
        tier: 1,
        places: &[(Spring, 30, 6)],
        stars: &[
            s("Spica", 1.0, 0, -2),
            s("Porrima", 2.7, 0, 1),
            s("Minelauva", 3.4, -1, 2),
            s("Vindemiatrix", 2.8, -2, 4),
        ],
        lore: "The largest constellation of the zodiac. Spica, her brightest star, is really two hot blue stars circling each other every 4 days.",
    },
    ConDef {
        id: "cancer",
        name: "Cancer",
        figure: "the crab",
        tier: 2,
        places: &[(Spring, 44, 12)],
        stars: &[
            s("Tarf", 3.5, 0, -2),
            s("Asellus Australis", 3.9, 0, 1),
            s("Asellus Borealis", 4.7, 0, 3),
            s("Acubens", 4.3, 2, -1),
        ],
        lore: "Faint and gentle. Its two donkey stars, the Aselli, stand either side of the Beehive cluster, as if feeding at a manger.",
    },
    // Summer.
    ConDef {
        id: "lyra",
        name: "Lyra",
        figure: "the lyre",
        tier: 0,
        places: &[(Summer, 22, 15)],
        stars: &[
            s("Vega", 0.0, 0, 0),
            s("the Double Double", 4.7, 2, 1),
            s("Sulafat", 3.3, 1, -3),
            s("Sheliak", 3.5, -1, -3),
        ],
        lore: "It belonged to Orpheus in the old myth. Vega, one of the brightest stars of all, was the pole star about 12 thousand years ago, and will be again.",
    },
    ConDef {
        id: "cygnus",
        name: "Cygnus",
        figure: "the swan",
        tier: 0,
        places: &[(Summer, 16, 11)],
        stars: &[
            s("Deneb", 1.25, 0, 3),
            s("Sadr", 2.2, 1, 1),
            s("Albireo", 3.1, 3, -3),
            s("Gienah", 2.5, -1, 0),
            s("Fawaris", 2.9, 3, 2),
        ],
        lore: "It flies down the Milky Way, and is also called the Northern Cross. Albireo, at its beak, looks like one star, but a small telescope shows two, one gold and one blue.",
    },
    ConDef {
        id: "aquila",
        name: "Aquila",
        figure: "the eagle",
        tier: 1,
        places: &[(Summer, 28, 9)],
        stars: &[
            s("Tarazed", 2.7, -1, 1),
            s("Altair", 0.8, 0, 0),
            s("Alshain", 3.7, 1, -1),
        ],
        lore: "The eagle. Altair spins so fast, about once every 9 hours, that it bulges at its middle. With Vega and Deneb it makes the Summer Triangle.",
    },
    ConDef {
        id: "scorpius",
        name: "Scorpius",
        figure: "the scorpion",
        tier: 1,
        places: &[(Summer, 36, 3)],
        stars: &[
            s("Acrab", 2.6, 3, 3),
            s("Dschubba", 2.3, 2, 2),
            s("Antares", 1.0, 1, 1),
            s("Sargas", 1.9, 0, -1),
            s("Shaula", 1.6, -2, -1),
            s("Lesath", 2.7, -3, 0),
        ],
        lore: "It hangs low in the summer south. Antares, its red heart, is so huge that in the Sun's place it would swallow the orbit of Mars. Its name means rival of Mars.",
    },
    ConDef {
        id: "sagittarius",
        name: "Sagittarius",
        figure: "the archer",
        tier: 2,
        places: &[(Summer, 28, 4)],
        stars: &[
            s("Kaus Borealis", 2.8, 1, 2),
            s("Kaus Media", 2.7, 1, 0),
            s("Kaus Australis", 1.8, 1, -2),
            s("Ascella", 2.6, -1, -2),
            s("Nunki", 2.0, -1, 0),
        ],
        lore: "Its brightest stars make the shape of a teapot. The centre of our galaxy lies just beyond its spout, behind thick clouds of dust.",
    },
    // Autumn.
    ConDef {
        id: "pegasus",
        name: "Pegasus",
        figure: "the winged horse",
        tier: 0,
        places: &[(Autumn, 30, 11)],
        stars: &[
            s("Algenib", 2.8, -2, -2),
            s("Markab", 2.5, 2, -2),
            s("Scheat", 2.4, 2, 2),
            s("Enif", 2.4, 5, -3),
        ],
        lore: "The winged horse. Its Great Square is so wide and empty that stargazers count the faint stars inside it to judge how dark their sky is.",
    },
    ConDef {
        id: "andromeda",
        name: "Andromeda",
        figure: "the chained princess",
        tier: 1,
        places: &[(Autumn, 25, 14)],
        stars: &[
            s("Alpheratz", 2.1, 3, -1),
            s("Mirach", 2.1, 0, 0),
            s("Almach", 2.1, -3, 1),
        ],
        lore: "She stretches from a corner of the Great Square. Near Mirach glows the Andromeda Galaxy, our great neighbour in space.",
    },
    ConDef {
        id: "aries",
        name: "Aries",
        figure: "the ram",
        tier: 2,
        places: &[(Autumn, 20, 9)],
        stars: &[
            s("Hamal", 2.0, 0, 1),
            s("Sheratan", 2.6, 1, -1),
            s("Mesarthim", 3.9, 1, -2),
        ],
        lore: "A short crooked line of stars. Over two thousand years ago the Sun crossed the spring equinox here, and astronomers still call that point the First Point of Aries.",
    },
];

/// Every wonder, by the seasons it is up.
pub const WONDERS: &[WonderDef] = &[
    WonderDef {
        id: "mizar_alcor",
        name: "Mizar and Alcor",
        line: "The horse and rider, two stars side by side in the Big Dipper's handle. Seeing both was once called a test of good eyes.",
        places: &[(Winter, 11, 8), (Spring, 8, 16), (Summer, 68, 11), (Autumn, 0, 6)],
    },
    WonderDef {
        id: "orion_nebula",
        name: "the Orion Nebula",
        line: "A glowing cloud where new stars are being born, hanging below Orion's belt like a sword.",
        places: &[(Winter, 36, 7)],
    },
    WonderDef {
        id: "pleiades",
        name: "the Pleiades",
        line: "The Seven Sisters, a little cluster of young blue stars. Most people can count six of them by eye.",
        places: &[(Winter, 46, 14)],
    },
    WonderDef {
        id: "beehive",
        name: "the Beehive cluster",
        line: "A soft swarm of hundreds of stars, like a faint cloud between the two donkey stars of Cancer.",
        places: &[(Spring, 43, 14)],
    },
    WonderDef {
        id: "ring_nebula",
        name: "the Ring Nebula",
        line: "A smoke ring of glowing gas, thrown off by a dying star, between Sheliak and Sulafat.",
        places: &[(Summer, 22, 12)],
    },
    WonderDef {
        id: "milky_way",
        name: "the heart of the Milky Way",
        line: "The thickest star clouds of our galaxy, rising like steam from the spout of the Teapot.",
        places: &[(Summer, 31, 5)],
    },
    WonderDef {
        id: "andromeda_galaxy",
        name: "the Andromeda Galaxy",
        line: "The farthest thing most people can see by eye. The light reaching you tonight left it two and a half million years ago.",
        places: &[(Autumn, 25, 16)],
    },
    WonderDef {
        id: "double_cluster",
        name: "the Double Cluster",
        line: "Two crowds of young stars side by side, between Perseus and Cassiopeia.",
        places: &[(Autumn, 6, 11)],
    },
];

/// Where the full moon rises: low in the east.
pub const MOON_AT: (i32, i32) = (18, 2);

/// Faint stars scattered through each night's sky.
pub const FAINT_STARS: usize = 30;

/// The constellations up in `season`'s evening sky, in catalog order.
pub fn season_constellations(season: Season) -> Vec<usize> {
    (0..CONSTELLATIONS.len())
        .filter(|&i| CONSTELLATIONS[i].centre(season).is_some())
        .collect()
}

/// The wonders up in `season`.
pub fn season_wonders(season: Season) -> Vec<usize> {
    (0..WONDERS.len())
        .filter(|&i| WONDERS[i].at(season).is_some())
        .collect()
}

/// The highest tier open with `logged` constellations in the journal.
pub fn open_tier(logged: usize) -> u8 {
    match logged {
        0..=1 => 0,
        2..=4 => 1,
        _ => 2,
    }
}

/// Constellations in the journal needed to open `tier`.
pub fn tier_needs(tier: u8) -> usize {
    match tier {
        0 => 0,
        1 => 2,
        _ => 5,
    }
}

/// A constellation's index by its id.
pub fn con_index(id: &str) -> Option<usize> {
    CONSTELLATIONS.iter().position(|c| c.id == id)
}

/// What sits on one step of the sky.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    /// Star `star` of constellation `con`.
    Star {
        con: usize,
        star: usize,
    },
    /// A faint star in no pattern.
    Faint,
    Wonder(usize),
    Moon,
}

/// One thing in tonight's sky.
#[derive(Clone, Debug)]
pub struct Object {
    pub az: i32,
    pub alt: i32,
    pub body: Body,
    pub mag: f32,
    /// Seconds to its next twinkle.
    pub twinkle: f64,
}

impl Object {
    pub fn at(&self) -> (i32, i32) {
        (self.az, self.alt)
    }
}

/// Tonight's sky: what is up, and where.
#[derive(Clone, Debug)]
pub struct Sky {
    pub season: Season,
    pub objects: Vec<Object>,
    /// The constellations up tonight, in the order they rose.
    pub up: Vec<usize>,
    pub full_moon: bool,
}

impl Sky {
    /// The `season`'s sky with constellations `up`, its wonders, the moon
    /// when full, and faint stars placed by `rng` away from every star the
    /// season could hold.
    pub fn new(season: Season, up: &[usize], full_moon: bool, rng: &mut Rng) -> Sky {
        let mut sky = Sky {
            season,
            objects: Vec::new(),
            up: Vec::new(),
            full_moon,
        };
        for &c in up {
            sky.raise(c, rng);
        }
        for w in season_wonders(season) {
            if let Some((az, alt)) = WONDERS[w].at(season) {
                sky.objects.push(Object {
                    az,
                    alt,
                    body: Body::Wonder(w),
                    mag: 4.0,
                    twinkle: 1.0 + rng.unit() * 2.0,
                });
            }
        }
        if full_moon {
            sky.objects.push(Object {
                az: MOON_AT.0,
                alt: MOON_AT.1,
                body: Body::Moon,
                mag: -12.0,
                twinkle: 2.0,
            });
        }
        let reserved = reserved_cells(season);
        let faint = if full_moon {
            FAINT_STARS / 3
        } else {
            FAINT_STARS
        };
        let mut placed = 0;
        let mut tries = 0;
        while placed < faint && tries < 2000 {
            tries += 1;
            let az = rng.below(AZ_STEPS as u32) as i32;
            let alt = rng.range(2, ZENITH - 2);
            if reserved.contains(&(az, alt)) || sky.object_at((az, alt)).is_some() {
                continue;
            }
            sky.objects.push(Object {
                az,
                alt,
                body: Body::Faint,
                mag: 4.0 + rng.unit() as f32 * 1.5,
                twinkle: rng.unit() * 3.0,
            });
            placed += 1;
        }
        sky
    }

    /// Puts constellation `c` in the sky, if it is not up already.
    pub fn raise(&mut self, c: usize, rng: &mut Rng) {
        if self.up.contains(&c) {
            return;
        }
        let def = &CONSTELLATIONS[c];
        for (i, star) in def.stars.iter().enumerate() {
            if let Some((az, alt)) = def.star_at(self.season, i) {
                self.objects.push(Object {
                    az,
                    alt,
                    body: Body::Star { con: c, star: i },
                    mag: star.mag,
                    twinkle: rng.unit() * 2.5,
                });
            }
        }
        self.up.push(c);
    }

    pub fn object_at(&self, at: (i32, i32)) -> Option<&Object> {
        self.objects.iter().find(|o| o.at() == at)
    }

    /// Where star `star` of `con` is tonight.
    pub fn star_place(&self, con: usize, star: usize) -> Option<(i32, i32)> {
        CONSTELLATIONS[con].star_at(self.season, star)
    }
}

/// Every step any star or wonder of `season` could stand on, kept clear of
/// faint stars, with the moon's.
pub fn reserved_cells(season: Season) -> Vec<(i32, i32)> {
    let mut cells = vec![MOON_AT];
    for c in season_constellations(season) {
        for i in 0..CONSTELLATIONS[c].stars.len() {
            if let Some(at) = CONSTELLATIONS[c].star_at(season, i) {
                cells.push(at);
            }
        }
    }
    for w in season_wonders(season) {
        if let Some(at) = WONDERS[w].at(season) {
            cells.push(at);
        }
    }
    cells
}

/// How loud a star of magnitude `mag` chimes, from 0.2 to 1.
pub fn brightness(mag: f32) -> f32 {
    (1.0 - mag * 0.18).clamp(0.2, 1.0)
}

/// "a bright star", "a star", "a dim star", for the words on resting.
pub fn brightness_words(mag: f32) -> &'static str {
    if mag < 1.0 {
        "one of the brightest stars"
    } else if mag < 2.5 {
        "a bright star"
    } else {
        "a gentle star"
    }
}
