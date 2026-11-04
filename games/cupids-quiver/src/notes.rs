//! The love notes that drift over the rooftops, folded small. Each one an
//! arrow brings down is kept, in the order they were found.

/// Every note, as the player hears it.
pub const NOTES: [&str; 24] = [
    "In neat pencil: Meet me where the pigeons sleep.",
    "In blue ink, smudged: I saved you the last cherry bun.",
    "On the back of a bus ticket: Same seat tomorrow?",
    "In a careful hand: Your laugh carries all the way up here.",
    "Folded into a tiny square: I wrote your name in the frost on my window.",
    "In red crayon: You are my favourite person in this whole town.",
    "On the corner of a map: Every road here leads back to you.",
    "Typed, with one letter crossed out: Dinner at eight. Wear the green scarf.",
    "In silver ink: The stars are out. So am I. Look up.",
    "On a recipe card: Two cups of flour, one of sugar, and all of my heart.",
    "In shaky letters: I practised saying this for a week. I like you.",
    "On sheet music: This one is yours. I play it every evening at six.",
    "In tidy capitals that soften at the end: Thank you for the umbrella.",
    "On a seed packet: Plant these with me in spring.",
    "In fountain pen: Fifty years, and I would marry you again tomorrow.",
    "On a ticket stub: Second row. Save the aisle seat for me.",
    "In pencil, rubbed out twice: Do you still walk the river path on Sundays?",
    "In green ink: The bakery cat likes you. So do I.",
    "On a paper napkin: You looked up just as I did. I keep thinking about it.",
    "On a library card: I borrowed your favourite book so I could talk to you.",
    "In a child's big letters: Mum, you are the best. Love from both of us.",
    "On brown paper: The kettle is on. Come home soon.",
    "In faded ink, very old: I kept every one of your letters.",
    "Written round the edge in a spiral: Yes. Yes. Always yes.",
];

/// The next note not yet found, as an index, or `None` once all are kept.
pub fn next_unfound(found: &[u8]) -> Option<u8> {
    (0..NOTES.len() as u8).find(|i| !found.contains(i))
}
