#[cfg(wickets_secrets)]
include!("../../../secrets/overlay/game-snowfall-skate-tests.rs");

#[cfg(not(wickets_secrets))]
include!("public.rs");
