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

/// Whether chunk mesh wireframes (`render::debug`) are visible from the
/// start, rather than needing
/// [`crate::config::input::TOGGLE_WIREFRAME`] pressed first.
///
/// Same reasoning as [`CHUNK_LOCK_INITIALLY_ENGAGED`]: only takes effect
/// while [`TESTING_TOOLS_ENABLED`] is also `true`, for consistency with
/// every other testing feature here, even though wireframes have no
/// "stuck forever" failure mode the way a permanently locked world would.
#[cfg(debug_assertions)]
pub const WIREFRAME_INITIALLY_VISIBLE: bool = false;

/// The chunk-bounds grid (`render::debug`) `render::debug` can draw around
/// whichever chunk the camera is in.
///
/// A settings-surface enum like [`crate::config::input::SprintMode`]: the
/// type lives here, as a value someone would tune; the behaviour of
/// cycling between them belongs to `render::debug`, which owns it.
#[cfg(debug_assertions)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ChunkGridMode {
    /// Nothing drawn.
    #[default]
    None,
    /// A wireframe cube around the chunk's bounds.
    Outline,
    /// The outline, plus a line through the centre along each axis, to find
    /// the middle at a glance.
    OutlineAndAxes,
}

/// Which [`ChunkGridMode`] `render::debug` starts in, rather than needing
/// [`crate::config::input::TOGGLE_CHUNK_GRID`] pressed first. Same
/// `TESTING_TOOLS_ENABLED` reasoning as the other `_INITIALLY_*` constants
/// here.
#[cfg(debug_assertions)]
pub const CHUNK_GRID_INITIAL_MODE: ChunkGridMode = ChunkGridMode::None;

/// Whether the chunk-bounds grid (`render::debug`) starts locked to
/// whichever chunk the camera is in the moment a world loads, rather than
/// needing [`crate::config::input::DEBUG_MODIFIER`] +
/// [`crate::config::input::TOGGLE_CHUNK_GRID`] pressed first.
///
/// Unlike the other `_INITIALLY_*` constants here, this can't simply be a
/// resource's `Default` — locking needs an actual chunk position, and none
/// exists until the world camera has spawned. `render::debug` applies it
/// the first frame a camera exists after entering `GameState::InGame`,
/// which in practice means once, the first time `InGame` is ever entered
/// this run — leaving and re-entering later doesn't re-lock. Good enough
/// for a testing convenience; press the hotkey again if you need it back.
#[cfg(debug_assertions)]
pub const CHUNK_GRID_INITIALLY_LOCKED: bool = false;

/// Whether the sea-level marker (`render::debug`) is visible from the
/// start, rather than needing
/// [`crate::config::input::TOGGLE_SEA_LEVEL_LINE`] pressed first. Same
/// `TESTING_TOOLS_ENABLED` reasoning as the other `_INITIALLY_*` constants
/// here.
#[cfg(debug_assertions)]
pub const SEA_LEVEL_LINE_INITIALLY_VISIBLE: bool = false;
