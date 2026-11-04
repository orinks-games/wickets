#[cfg(wickets_secrets)]
include!("../../../secrets/overlay/game-belfry-tests.rs");

#[cfg(not(wickets_secrets))]
include!("public.rs");
