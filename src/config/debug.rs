//! Testing-only toggles: switches that exist purely to help develop and
//! debug the game, never real gameplay.
//!
//! Everything here is compiled out of release entirely, the same as
//! [`crate::states::debug`]'s jump keys — a testing tool that could
//! theoretically reach a shipped build is one that was never gated tightly
//! enough.

/// Master switch for every testing feature's hotkey — chunk locking
/// (`world::debug`), and whatever joins it later.
///
/// Being a debug build is not the same as *actively testing* right now:
/// this stays independent of that so a stray keypress during ordinary
/// debug-build play can't silently trigger a testing feature. When `false`,
/// a testing hotkey does nothing at all — the system behind it is never
/// even added to the schedule, not merely skipped. Set it to `true` while
/// deliberately using a testing feature, back to `false` once done.
#[cfg(debug_assertions)]
pub const TESTING_TOOLS_ENABLED: bool = true;

/// Whether chunk locking (`world::debug`) starts engaged the moment a world
/// loads, rather than needing [`crate::config::input::TOGGLE_CHUNK_LOCK`]
/// pressed first.
///
/// Only takes effect while [`TESTING_TOOLS_ENABLED`] is also `true` —
/// otherwise the world would start locked with no hotkey able to unlock it,
/// since the hotkey itself is only registered when testing tools are
/// enabled. See `world::ChunkLock`.
#[cfg(debug_assertions)]
pub const CHUNK_LOCK_INITIALLY_ENGAGED: bool = false;
