//! Apply the link arguments Prism needs to the game's final link.
//! hearken-speech works out the arguments (none since Prism 0.1.4) and the
//! `hearken` facade passes them on; a link argument only reaches the final
//! link, which is this crate's, so every game passes them on here, and a
//! future argument reaches it with no change to the game.

fn main() {
    println!("cargo:rerun-if-env-changed=DEP_HEARKEN_LINK_ARGS");
    match std::env::var("DEP_HEARKEN_LINK_ARGS") {
        Ok(args) => {
            for arg in args.split(';').filter(|arg| !arg.is_empty()) {
                println!("cargo:rustc-link-arg={arg}");
            }
        }
        // Cargo hands a build script this list only from a direct dependency.
        Err(_) => println!(
            "cargo:warning=Prism's link arguments are missing. Fix: add hearken to this \
             package's [dependencies]."
        ),
    }
}
