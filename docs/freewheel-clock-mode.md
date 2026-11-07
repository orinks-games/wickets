# Freewheel: Against the clock

The next Freewheel milestone, after the endless ride, shop and job board (milestone 2, shipped in 1.1.1). It comes from a tester's idea, judged on 26 October 2026: deliver as much as you can against one clock, the way Crazy Taxi does it. Orinks put it ahead of milestone 3 (races).

It ships on the alpha channel first (`clock::STAGE`). Stable and beta copies show no row, help, sound check entry or records page for it.

## The mode

- One clock, starting at 60 seconds. It runs the whole ride, and the ride ends at zero. There is no crash limit and no finish line.
- The street is the endless ride's, in the district you chose, with its hazards closing up the further you go.
- Drops are the endless ride's parcels: a shop doorbell from the drop's side, then ride over it at 20 km/h or less.
- A doorway drop (in the door zone) adds 14 seconds and pays 3 dollars. A locker drop (by the centre line, among the cars) adds 18 and pays 6.
- A drop within 30 seconds of the last is a quick drop: 4 seconds more.
- The clock holds 90 seconds at most.
- A crash costs 8 seconds, on top of the 3 seconds spent down.
- Every drop has 40 metres of clear road before and after it. That is more than any bike covers in a warning lead, so a doorbell never rings while a hazard is warning. A test rides every district at three speeds to check this.
- Spoken: each drop says what it added and the time left ("Plus 18, quick drop. 47 left."). A crash says "Minus 8 seconds." The game says 30 and 10 seconds left. Under 15 seconds a dry tick sounds once a second, twice a second under 5.
- S: the first press adds the time left before speed and lane. The second gives drops and cash.
- Score: drops made, best kept per district (`clock/<district>` board). Cash is banked like the endless ride's, even when you leave early.
- Code: `games/freewheel/src/clock.rs` (the clock, the street, the rules); the world runs the clock, and the ride screen speaks it.

## Decisions for Orinks

Each one is built as the recommendation, so the PR plays as is. Say if you want any changed.

1. **Name.** "Against the clock". Not "rush" (rush hour already exists) or "shift" (kept for career work).
2. **Numbers.** Start 60, doorway +14, locker +18, quick +4 within 30 seconds, crash -8, cap 90. Drops come about every 120 metres, the first 30 metres after the start. A careful rider loses a few seconds per drop and lasts two to four minutes. A rider who takes lockers and keeps the drops quick can hold the clock up.
3. **Moped.** Allowed. It has to brake hard for every drop, and the help says so. The other choice is to leave it out of the mode.
4. **Call-outs.** Time lines ("Plus 14. 52 left.", "Minus 8 seconds.", "10 seconds left.") are always spoken, like crash lines, because they are the score. The doorbell is named under the Spoken call-outs setting, as in the endless ride.
5. **No new Settings row, no event day, no challenge.** The daily, weekly and monthly rides stay time trials.
6. **Later (PR 2 or 3, not in this PR).** The tester's second idea: a long job-board delivery where extra parcels on the way add time to the deadline. Also possible: a daily clock street that is the same for everyone.
