//! Every integration test of this crate, built as one test binary:
//! each linked binary carries SDL and Prism, so one link beats several.
//! Add a new test file here as a module.

mod music;
mod routes;
mod rules;
mod run;
mod walkthrough;
mod watch;
