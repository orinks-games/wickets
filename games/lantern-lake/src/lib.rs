#[cfg(wickets_secrets)]
include!("../../../secrets/overlay/game-lantern-lake.rs");

#[cfg(not(wickets_secrets))]
include!("public.rs");
