//! Defines the `vr` cfg, which is what everything VR-related is gated on.
//!
//! It means "the `vr` feature is on *and* this target can actually have it".
//! The VR path talks to the compositor over a DXGI shared texture and a named
//! pipe, so it is Windows only; everywhere else the feature has to quietly do
//! nothing rather than fail the build, because `--all-features` has to keep
//! working. Spelling that out at every one of the two dozen gates would be
//! noise, and getting it wrong at one of them would be a broken build on a
//! target nobody checks often.

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rustc-check-cfg=cfg(vr)");

    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    if std::env::var_os("CARGO_FEATURE_VR").is_some() && windows {
        println!("cargo::rustc-cfg=vr");
    }
}
