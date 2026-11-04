#[cfg(wickets_secrets)]
include!("../../../secrets/overlay/game-riverboat-dice.rs");

#[cfg(not(wickets_secrets))]
include!("public.rs");
