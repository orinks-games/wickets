fn main() {
    wickets_content::detect();
    println!("cargo:rerun-if-env-changed=DEP_HEARKEN_LINK_ARGS");
    if let Ok(args) = std::env::var("DEP_HEARKEN_LINK_ARGS") {
        for arg in args.split(';').filter(|arg| !arg.is_empty()) {
            println!("cargo:rustc-link-arg={arg}");
        }
    }
}
