//! The deal pool: seeds whose deals the solver in `solver.rs` has won,
//! within `POOL_LIMIT` tables each. Written by `pool_source` in the tests,
//! through the ignored test `write_the_deal_pool`:
//! `cargo test -p fireside-patience --release -- --ignored write_the_deal_pool`.
//! The tests solve every seed again.

/// Draw one deals, for relaxed play and the daily and monthly challenges.
pub const RELAXED: &[u64] = &[
    1, 2, 5, 8, 11, 16, 18, 21, 22, 23, 24, 26, 27, 28, 29, 30, 34, 36, 37, 38, 39, 40, 43, 44, 47,
    48, 49, 52, 53, 56, 58, 62, 64, 70, 72, 73, 74, 79, 80, 81, 85, 86, 90, 92, 94, 97, 98, 100,
    101, 102, 103, 105, 107, 108, 109, 110, 111, 112, 113, 115, 116, 118, 119, 120, 121, 122, 123,
    124, 126, 127, 129, 132, 133, 135, 137, 139, 140, 141, 142, 144, 145, 147, 148, 149, 151, 152,
    155, 160, 161, 162, 164, 165, 166, 169, 170, 171, 174, 177, 179, 180, 181, 182, 183, 184, 185,
    187, 188, 190, 191, 192, 193, 194, 196, 198, 199, 200, 203, 204, 205, 206,
];

/// Draw three deals, for the weekly challenge.
pub const DRAW_THREE: &[u64] = &[
    1, 5, 11, 13, 21, 22, 25, 26, 27, 28, 29, 30, 34, 36, 37, 38, 39, 40, 44, 48, 49, 52, 53, 56,
    57, 62, 63, 64, 70, 72, 73, 74, 77, 79, 81, 85, 86, 88, 90, 92,
];
