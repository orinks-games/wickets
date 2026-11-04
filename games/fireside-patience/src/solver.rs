//! A depth-first solver that sees every card, face down or not. It proves
//! a deal can be won and gives the moves that win it. It builds the deal
//! pool in `pool.rs` and checks it in the tests, and the hint follows the
//! line it finds from the table as it stands.
//!
//! The search plays safe foundation moves at once, tries the moves most
//! likely to help first, never visits a table twice, and gives up after a
//! set number of tables. A deal it gives up on may still be winnable: the
//! pool only keeps the deals it wins.

use std::collections::HashSet;

use crate::rules::{safe_to_put_up, stock_reach, Move, Spot, Step, Table, KING, PILES};

/// How many tables the pool's search may visit before it gives up.
pub const POOL_LIMIT: usize = 2_000;

/// The steps that win the deal on `table`, turning `draw` cards at a time,
/// or `None` when the search gives up within `limit` tables.
pub fn solve(table: &Table, draw: usize, limit: usize) -> Option<Vec<Step>> {
    solve_counted(table, draw, limit).0
}

/// `solve`, with the number of tables the search visited.
pub fn solve_counted(table: &Table, draw: usize, limit: usize) -> (Option<Vec<Step>>, usize) {
    let mut search = Search {
        draw,
        limit,
        nodes: 0,
        seen: HashSet::new(),
        path: Vec::new(),
    };
    let won = search.dfs(table.clone());
    (won.then_some(search.path), search.nodes)
}

struct Search {
    draw: usize,
    limit: usize,
    nodes: usize,
    seen: HashSet<u64>,
    path: Vec<Step>,
}

impl Search {
    fn dfs(&mut self, mut t: Table) -> bool {
        let mark = self.path.len();
        // Safe moves to the foundations cost nothing to take.
        while let Some(mv) = safe_move(&t) {
            t.apply(mv);
            self.path.push(Step::Move(mv));
        }
        if t.won() {
            return true;
        }
        if self.nodes >= self.limit || !self.seen.insert(t.hash64()) {
            self.path.truncate(mark);
            return false;
        }
        self.nodes += 1;
        for (steps, next) in candidates(&t, self.draw) {
            let before = self.path.len();
            self.path.extend(steps);
            if self.dfs(next) {
                return true;
            }
            self.path.truncate(before);
            if self.nodes >= self.limit {
                break;
            }
        }
        self.path.truncate(mark);
        false
    }
}

fn safe_move(t: &Table) -> Option<Move> {
    for p in 0..PILES {
        if let Some(card) = t.piles[p].top() {
            if safe_to_put_up(t, card) {
                return Some(Move::new(
                    Spot::Pile(p),
                    1,
                    Spot::Foundation(card.suit.index()),
                ));
            }
        }
    }
    let card = *t.waste.last()?;
    safe_to_put_up(t, card).then(|| Move::new(Spot::Waste, 1, Spot::Foundation(card.suit.index())))
}

/// The moves worth trying from `t`, with the table each leaves, best
/// first: up to the foundations, runs that turn a face-down card, cards
/// from the stock, then other pile moves.
fn candidates(t: &Table, draw: usize) -> Vec<(Vec<Step>, Table)> {
    let mut up = Vec::new();
    let mut flips = Vec::new();
    let mut stock = Vec::new();
    let mut other = Vec::new();
    let after = |t: &Table, mv: Move| {
        let mut next = t.clone();
        next.apply(mv);
        next
    };
    for p in 0..PILES {
        let pile = &t.piles[p];
        if let Some(card) = pile.top() {
            let mv = Move::new(Spot::Pile(p), 1, Spot::Foundation(card.suit.index()));
            if t.check(mv).is_ok() {
                up.push((vec![Step::Move(mv)], after(t, mv)));
            }
        }
        for count in 1..=pile.up.len() {
            let whole = count == pile.up.len();
            let first = pile.up[pile.up.len() - count];
            if !whole {
                // A part run is worth moving only to free a card to go up.
                let freed = pile.up[pile.up.len() - count - 1];
                if t.foundations[freed.suit.index()] + 1 != freed.rank {
                    continue;
                }
            }
            for q in 0..PILES {
                if q == p {
                    continue;
                }
                let target_empty = t.piles[q].is_empty();
                // A king already at the bottom of its pile has nowhere better to be.
                if whole && pile.down.is_empty() && target_empty && first.rank == KING {
                    continue;
                }
                let mv = Move::new(Spot::Pile(p), count, Spot::Pile(q));
                if t.check(mv).is_err() {
                    continue;
                }
                let entry = (vec![Step::Move(mv)], after(t, mv));
                if whole && !pile.down.is_empty() {
                    flips.push(entry);
                } else {
                    other.push(entry);
                }
                // One empty pile is as good as another.
                if target_empty {
                    break;
                }
            }
        }
    }
    for (turns, card, table) in stock_reach(t, draw) {
        let mut targets = vec![Spot::Foundation(card.suit.index())];
        targets.extend((0..PILES).map(Spot::Pile));
        let mut empty_tried = false;
        for to in targets {
            if let Spot::Pile(q) = to {
                if table.piles[q].is_empty() {
                    if empty_tried {
                        continue;
                    }
                    empty_tried = true;
                }
            }
            let mv = Move::new(Spot::Waste, 1, to);
            if table.check(mv).is_ok() {
                let mut steps = vec![Step::Turn; turns];
                steps.push(Step::Move(mv));
                stock.push((steps, after(&table, mv)));
            }
        }
    }
    up.into_iter()
        .chain(flips)
        .chain(stock)
        .chain(other)
        .collect()
}

/// The seeds from `start` on whose deals the solver wins within `limit`
/// tables, until it has `count` of them.
pub fn find_pool(start: u64, count: usize, draw: usize, limit: usize) -> Vec<u64> {
    let mut out = Vec::new();
    let mut seed = start;
    while out.len() < count {
        if solve(&Table::deal(seed), draw, limit).is_some() {
            out.push(seed);
        }
        seed += 1;
    }
    out
}

/// Seeds in the relaxed pool.
pub const RELAXED_SIZE: usize = 120;
/// Seeds in the draw three pool.
pub const DRAW_THREE_SIZE: usize = 40;
