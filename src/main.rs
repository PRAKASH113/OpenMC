//! Entry point. Declares the crate's top-level modules and hands off to
//! [`app::run`]; all setup lives in [`app`].

// Keep the console window in dev builds so logs are visible, hide it in
// release so the game opens as a normal windowed app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod camera;
mod ingame;
mod loading;
mod menu;
mod paused;

use bevy::prelude::AppExit;

fn main() -> AppExit {
    app::run()
}
