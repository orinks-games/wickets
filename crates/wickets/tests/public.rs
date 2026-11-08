// Every integration test of this crate, built as one test binary:
// each linked binary carries SDL and Prism, so one link beats several.
// Add a new test file here as a module.

mod challenge_rules;
mod discovery_clock;
mod lesson_sweep;
mod menu_sounds;
mod menus;
mod seasonal_discovery;
mod today;
mod visuals;
