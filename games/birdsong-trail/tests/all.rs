#[cfg(wickets_secrets)]
include!("../../../secrets/overlay/game-birdsong-trail-tests.rs");

#[cfg(not(wickets_secrets))]
include!("public.rs");
