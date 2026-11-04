//! Turn order for hot-seat play: two to four people taking turns on one
//! keyboard.
//!
//! [`Seats`] keeps whose turn it is and the round, and names each seat the
//! way the game says it: "Player 2", or a name the players chose. The turn
//! line, "Player 2's turn.", leads with the name, so a player who cuts the
//! speech short still hears whose turn it is.

/// The seats at a hot-seat table, and whose turn it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seats {
    names: Vec<String>,
    current: usize,
    round: u32,
}

impl Seats {
    /// `count` seats named "Player 1", "Player 2" and so on. Seat 0 plays
    /// first, in round 1. Panics when `count` is 0.
    pub fn new(count: usize) -> Self {
        Seats::named((1..=count).map(|n| format!("Player {n}")))
    }

    /// One seat per name, in turn order. Panics when there are none.
    pub fn named<S: Into<String>>(names: impl IntoIterator<Item = S>) -> Self {
        let names: Vec<String> = names.into_iter().map(Into::into).collect();
        assert!(!names.is_empty(), "Seats needs at least one seat");
        Seats {
            names,
            current: 0,
            round: 1,
        }
    }

    /// How many seats there are.
    pub fn count(&self) -> usize {
        self.names.len()
    }

    /// The seat whose turn it is, from 0.
    pub fn current(&self) -> usize {
        self.current
    }

    /// The round, from 1. It goes up each time play passes the last seat.
    pub fn round(&self) -> u32 {
        self.round
    }

    /// A seat's spoken name: "Player 2". Panics on a seat out of range.
    pub fn name(&self, seat: usize) -> &str {
        &self.names[seat]
    }

    /// The name of the seat whose turn it is.
    pub fn current_name(&self) -> &str {
        self.name(self.current)
    }

    /// What the game says when a turn starts: "Player 2's turn."
    pub fn turn_line(&self) -> String {
        format!("{}'s turn.", self.current_name())
    }

    /// Passes the turn to the next seat, wrapping to seat 0 and the next
    /// round after the last. Returns the new seat.
    pub fn pass(&mut self) -> usize {
        self.current += 1;
        if self.current == self.names.len() {
            self.current = 0;
            self.round += 1;
        }
        self.current
    }

    /// Passes the turn to the next seat that `playing` accepts, for seats
    /// that are out or have stood up. Returns `None`, with the turn left
    /// where it was, when no seat is playing.
    pub fn pass_to_playing(&mut self, playing: impl Fn(usize) -> bool) -> Option<usize> {
        let before = self.clone();
        for _ in 0..self.names.len() {
            let seat = self.pass();
            if playing(seat) {
                return Some(seat);
            }
        }
        *self = before;
        None
    }

    /// Every seat once, starting with the one whose turn it is: the order
    /// a status line reads the table in.
    pub fn order(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.names.len()).map(move |i| (self.current + i) % self.names.len())
    }

    /// Back to seat 0 in round 1, for a new game with the same players.
    pub fn restart(&mut self) {
        self.current = 0;
        self.round = 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seats_rotate_and_name_players() {
        let mut seats = Seats::new(3);
        assert_eq!(seats.count(), 3);
        assert_eq!(seats.turn_line(), "Player 1's turn.");
        assert_eq!(seats.pass(), 1);
        assert_eq!(seats.turn_line(), "Player 2's turn.");
        assert_eq!(seats.round(), 1);
        seats.pass();
        assert_eq!(seats.current_name(), "Player 3");
        assert_eq!(seats.pass(), 0);
        assert_eq!(seats.round(), 2);
        assert_eq!(seats.turn_line(), "Player 1's turn.");
        seats.pass();
        assert_eq!(seats.order().collect::<Vec<_>>(), vec![1, 2, 0]);
        seats.restart();
        assert_eq!((seats.current(), seats.round()), (0, 1));
    }

    #[test]
    fn seats_take_chosen_names() {
        let mut seats = Seats::named(["Ana", "Ben"]);
        assert_eq!(seats.turn_line(), "Ana's turn.");
        seats.pass();
        assert_eq!(seats.turn_line(), "Ben's turn.");
        assert_eq!(seats.name(0), "Ana");
    }

    #[test]
    fn seats_skip_players_who_are_out() {
        let mut seats = Seats::new(4);
        let out = [false, true, true, false];
        assert_eq!(seats.pass_to_playing(|s| !out[s]), Some(3));
        assert_eq!(seats.pass_to_playing(|s| !out[s]), Some(0));
        assert_eq!(seats.round(), 2);
        assert_eq!(seats.pass_to_playing(|_| false), None);
        assert_eq!((seats.current(), seats.round()), (0, 2));
    }

    #[test]
    #[should_panic(expected = "at least one seat")]
    fn seats_need_a_seat() {
        Seats::new(0);
    }
}
