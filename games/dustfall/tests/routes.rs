//! The route finder against hand-drawn grids: every raider's habits, walls
//! that force bashing, and the same answer every time.

use dustfall::rules::grid::{Grid, Sector, Work, WorkKind, COLS, WALL_HITS};
use dustfall::rules::raiders::{Kind, Raider};
use dustfall::rules::route::{self, Costs, Goal};

fn s(col: u8, row: u8) -> Sector {
    Sector::new(col, row)
}

fn raider(kind: Kind, at: Sector) -> Raider {
    Raider::new(1, kind, at)
}

fn plan(grid: &Grid, kind: Kind, at: Sector, gun: u8) -> Vec<Sector> {
    let goal = route::goal(kind, &[], &[]);
    route::plan(grid, &raider(kind, at), &goal, gun).expect("a route")
}

fn wall_row(grid: &mut Grid, row: u8) {
    for col in 0..COLS {
        grid.set(s(col, row), Some(Work::Wall { hits: WALL_HITS }));
    }
}

#[test]
fn an_open_grid_is_walked_straight_down() {
    let grid = Grid::new();
    let route = plan(&grid, Kind::Drifter, s(0, 6), 2);
    assert_eq!(route, vec![s(0, 5), s(0, 4), s(0, 3), s(0, 2), s(0, 1)]);
    let route = plan(&grid, Kind::Hulk, s(4, 6), 2);
    assert_eq!(route, vec![s(4, 5), s(4, 4), s(4, 3), s(4, 2), s(4, 1)]);
}

#[test]
fn a_drifter_goes_round_a_wall() {
    let mut grid = Grid::new();
    grid.set(s(0, 3), Some(WorkKind::Wall.build()));
    let route = plan(&grid, Kind::Drifter, s(0, 6), 2);
    assert!(route.iter().all(|at| !grid.is_wall(*at)), "{route:?}");
    assert_eq!(route.last().map(|at| at.row), Some(1));
    assert_eq!(route.len(), 6, "one sidestep: {route:?}");
}

#[test]
fn a_drifter_goes_round_flak_cover_when_the_detour_is_short() {
    let mut grid = Grid::new();
    grid.set(s(0, 3), Some(WorkKind::Flak.build()));
    let route = plan(&grid, Kind::Drifter, s(0, 6), 4);
    assert!(
        route.iter().all(|at| !grid.flak_covers(*at)),
        "the drifter walked under the post: {route:?}"
    );
    assert_eq!(route.last().map(|at| at.row), Some(1));
}

#[test]
fn a_drifter_walks_through_cover_when_the_detour_is_long() {
    // Posts across the three left columns, far from the far right.
    let mut grid = Grid::new();
    for col in 0..3 {
        grid.set(s(col, 3), Some(WorkKind::Flak.build()));
    }
    let route = plan(&grid, Kind::Drifter, s(0, 6), 2);
    // Round them is four sideways and five down, 9 steps; through costs
    // 5 steps plus 6 of cover, 11. It goes round.
    assert_eq!(route.len(), 9, "{route:?}");
    assert!(route.iter().all(|at| !grid.flak_covers(*at)), "{route:?}");
    // Posts in every column: nowhere is clear, so it walks straight.
    let mut grid = Grid::new();
    for col in 0..COLS {
        grid.set(s(col, 3), Some(WorkKind::Flak.build()));
    }
    let route = plan(&grid, Kind::Drifter, s(0, 6), 2);
    assert_eq!(route.len(), 5, "nowhere to go round: {route:?}");
}

#[test]
fn an_unpowered_post_covers_nothing() {
    let mut grid = Grid::new();
    grid.set(s(0, 3), Some(Work::Flak { powered: false }));
    let route = plan(&grid, Kind::Drifter, s(0, 6), 4);
    assert_eq!(route.len(), 5, "{route:?}");
}

#[test]
fn a_hulk_ignores_flak_cover() {
    let mut grid = Grid::new();
    grid.set(s(0, 3), Some(WorkKind::Flak.build()));
    let route = plan(&grid, Kind::Hulk, s(0, 6), 4);
    assert_eq!(route, vec![s(0, 5), s(0, 4), s(0, 3), s(0, 2), s(0, 1)]);
}

#[test]
fn a_hulk_bashes_through_when_that_is_shorter() {
    // A wall in its column, and the columns beside it walled too, so going
    // round is three sideways and back: bashing is cheaper.
    let mut grid = Grid::new();
    for col in 0..4 {
        grid.set(s(col, 3), Some(WorkKind::Wall.build()));
    }
    let route = plan(&grid, Kind::Hulk, s(0, 6), 2);
    assert!(route.contains(&s(0, 3)), "{route:?}");
    assert_eq!(route.len(), 5);
    // One wall alone is walked round: five steps and one sideways beats
    // two bashes.
    let mut grid = Grid::new();
    grid.set(s(2, 3), Some(WorkKind::Wall.build()));
    let route = plan(&grid, Kind::Hulk, s(2, 6), 0);
    assert!(!route.contains(&s(2, 3)), "{route:?}");
}

#[test]
fn a_full_row_of_walls_is_bashed_by_everyone() {
    let mut grid = Grid::new();
    wall_row(&mut grid, 3);
    for kind in [Kind::Drifter, Kind::Skitter, Kind::Hulk, Kind::Sapper] {
        let route = plan(&grid, kind, s(2, 6), 4);
        let walls = route.iter().filter(|at| grid.is_wall(**at)).count();
        assert_eq!(walls, 1, "{kind:?} should bash exactly one wall: {route:?}");
        assert_eq!(route.last().map(|at| at.row), Some(1));
    }
}

#[test]
fn a_skitter_slips_to_the_side_the_gun_is_not_on() {
    let grid = Grid::new();
    let goal = Goal::NearRow;
    let mut left = raider(Kind::Skitter, s(2, 6));
    left.lean = -1;
    let route = route::plan(&grid, &left, &goal, 2).unwrap();
    assert_eq!(route.last(), Some(&s(1, 1)), "{route:?}");
    let mut right = left.clone();
    right.lean = 1;
    let route = route::plan(&grid, &right, &goal, 2).unwrap();
    assert_eq!(route.last(), Some(&s(3, 1)), "{route:?}");
    // With the gun elsewhere it goes straight down.
    let route = route::plan(&grid, &left, &goal, 4).unwrap();
    assert_eq!(route.last(), Some(&s(2, 1)), "{route:?}");
}

#[test]
fn a_sapper_heads_for_hydroponics_first() {
    let grid = Grid::new();
    let goal = route::goal(Kind::Sapper, &[1, 4], &[4]);
    assert_eq!(goal, Goal::Columns(vec![4]));
    let route = route::plan(&grid, &raider(Kind::Sapper, s(0, 6)), &goal, 2).unwrap();
    assert_eq!(route.last(), Some(&s(4, 1)));
    // Any building when there are no hydroponics.
    let goal = route::goal(Kind::Sapper, &[1, 3], &[]);
    let route = route::plan(&grid, &raider(Kind::Sapper, s(0, 6)), &goal, 2).unwrap();
    assert_eq!(route.last(), Some(&s(1, 1)));
    // The near row anywhere when there is nothing to sabotage.
    assert_eq!(route::goal(Kind::Sapper, &[], &[]), Goal::NearRow);
}

#[test]
fn a_mothership_comes_to_its_post_and_stops() {
    let grid = Grid::new();
    let goal = route::goal(Kind::Mothership, &[], &[]);
    assert_eq!(goal, Goal::At(s(2, 3)));
    let route = route::plan(&grid, &raider(Kind::Mothership, s(2, 6)), &goal, 2).unwrap();
    assert_eq!(route, vec![s(2, 5), s(2, 4), s(2, 3)]);
    let route = route::plan(&grid, &raider(Kind::Mothership, s(2, 3)), &goal, 2).unwrap();
    assert!(route.is_empty());
}

#[test]
fn raiders_never_step_back_toward_the_horizon() {
    // A pocket: walls either side and below, open only upward. The raider
    // bashes down rather than climbing out.
    let mut grid = Grid::new();
    grid.set(s(1, 4), Some(WorkKind::Wall.build()));
    grid.set(s(3, 4), Some(WorkKind::Wall.build()));
    grid.set(s(2, 3), Some(WorkKind::Wall.build()));
    let route = plan(&grid, Kind::Drifter, s(2, 4), 0);
    assert!(route.iter().all(|at| at.row <= 4), "{route:?}");
    assert!(route.iter().any(|at| grid.is_wall(*at)), "{route:?}");
}

#[test]
fn the_same_grid_gives_the_same_route() {
    let mut grid = Grid::new();
    grid.set(s(1, 4), Some(WorkKind::Wall.build()));
    grid.set(s(3, 2), Some(WorkKind::Flak.build()));
    let first = plan(&grid, Kind::Drifter, s(2, 6), 1);
    for _ in 0..20 {
        assert_eq!(plan(&grid, Kind::Drifter, s(2, 6), 1), first);
    }
}

#[test]
fn ties_go_down_before_sideways() {
    let grid = Grid::new();
    // A goal column two over: the route steps down first, then across,
    // the same way each time.
    let goal = Goal::Columns(vec![2]);
    let route = route::find(&grid, s(0, 6), &goal, Costs::open(Kind::Drifter), 4, 0).unwrap();
    assert_eq!(route[0], s(0, 5), "{route:?}");
    assert_eq!(route.last(), Some(&s(2, 1)));
    assert_eq!(route.len(), 7);
}

#[test]
fn landing_phases_follow_the_pace() {
    assert_eq!(route::phases_to_land(Kind::Drifter, false, 5), 5);
    assert_eq!(route::phases_to_land(Kind::Skitter, false, 5), 3);
    assert_eq!(route::phases_to_land(Kind::Hulk, false, 5), 9);
    assert_eq!(route::phases_to_land(Kind::Hulk, true, 5), 10);
    assert_eq!(route::phases_to_land(Kind::Drifter, false, 0), 0);
}
