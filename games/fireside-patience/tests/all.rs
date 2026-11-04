#[cfg(wickets_secrets)]
include!("../../../secrets/overlay/game-fireside-patience-tests.rs");

#[cfg(not(wickets_secrets))]
include!("public.rs");
