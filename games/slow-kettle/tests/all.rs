#[cfg(wickets_secrets)]
include!("../../../secrets/overlay/game-slow-kettle-tests.rs");

#[cfg(not(wickets_secrets))]
include!("public.rs");
