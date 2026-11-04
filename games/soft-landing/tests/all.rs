#[cfg(wickets_secrets)]
include!("../../../secrets/overlay/game-soft-landing-tests.rs");

#[cfg(not(wickets_secrets))]
include!("public.rs");
