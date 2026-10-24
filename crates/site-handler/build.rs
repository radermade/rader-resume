//! Declares the build-time inputs baked into the binary.
//!
//! Without these, cargo will not rebuild when the values change and a cached
//! build silently serves a stale commit SHA.
fn main() {
    println!("cargo::rerun-if-env-changed=GIT_SHA");
    println!("cargo::rerun-if-env-changed=BUILD_MONTH");
    println!("cargo::rerun-if-env-changed=SITE_URL");
}
