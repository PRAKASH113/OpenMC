//! Entry point. Declares the crate's top-level modules and hands off to
//! [`app::run`]; all setup lives in [`app`].

// Keep the console window in dev builds so logs are visible, hide it in
// release so the game opens as a normal windowed app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// No `unsafe` anywhere in the crate. A dependency that genuinely needs it
// gets a small wrapper module with a local `#[allow(unsafe_code)]` and a
// comment saying why — never a lifted crate-wide deny.
//
// This would normally sit in Cargo.toml as `[lints.rust]`, but the "Even
// Better TOML" editor extension mis-evaluates the cross-file schema reference
// for that table and shows a false error to anyone who opens the manifest. As
// an attribute it behaves identically for this single-crate project. Move it
// back once integration tests (`tests/`), benches, or a workspace appear —
// those are separate crates that only `[lints]` would cover.
#![deny(unsafe_code)]

mod app;
mod camera;
mod config;
mod input;
mod states;
mod utils;
mod window;

use bevy::prelude::AppExit;

fn main() -> AppExit {
    app::run()
}
