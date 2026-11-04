#[cfg(wickets_secrets)]
include!("../../../secrets/overlay/launcher-tests.rs");

#[cfg(not(wickets_secrets))]
include!("public.rs");
