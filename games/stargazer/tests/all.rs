#[cfg(wickets_secrets)]
include!("../../../secrets/overlay/game-stargazer-tests.rs");

#[cfg(not(wickets_secrets))]
include!("public.rs");
