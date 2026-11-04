#[cfg(wickets_secrets)]
include!("../../../secrets/overlay/game-rooftop-garden-tests.rs");

#[cfg(not(wickets_secrets))]
include!("public.rs");
