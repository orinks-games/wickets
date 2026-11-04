#[cfg(wickets_secrets)]
include!("../../../secrets/overlay/game-stargazer.rs");

#[cfg(not(wickets_secrets))]
include!("public.rs");
