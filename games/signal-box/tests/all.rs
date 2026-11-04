#[cfg(wickets_secrets)]
include!("../../../secrets/overlay/game-signal-box-tests.rs");

#[cfg(not(wickets_secrets))]
include!("public.rs");
