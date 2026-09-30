//! Player settings: game mode, gravity, jumping, and the body's size.
//!
//! Plain values, like the rest of `config/`. Movement speeds and the keys
//! stay in [`crate::config::input`] with the other control settings.

/// The two sets of rules the player can play under.
///
/// A settings-surface enum like [`crate::config::input::SprintMode`]: the
/// type and the starting value live here, and what each mode *does* belongs
/// to `player`. The names are placeholders borrowed from Minecraft until this
/// game settles on its own. Renaming is a change to this enum and its uses,
/// nothing structural.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameMode {
    /// Gravity always applies, there's no flight, and the testing-tool keys
    /// are switched off.
    Survival,
    /// Gravity applies until the player takes off: double-tap
    /// [`crate::config::input::UP`] to fly, double-tap
    /// [`crate::config::input::DOWN`] to land again. Testing-tool keys work.
    Creative,
}

/// Which [`GameMode`] the game starts in.
pub const INITIAL_GAME_MODE: GameMode = GameMode::Creative;

/// Whether entering [`GameMode::Creative`] starts the player already flying,
/// rather than on foot waiting for a double-tap of Space. Applies every time
/// Creative is entered, whether at startup or through
/// [`crate::config::input::TOGGLE_GAME_MODE`].
pub const CREATIVE_STARTS_FLYING: bool = false;

/// Downward acceleration while not flying, in blocks per second squared.
/// Minecraft's is about 32.
pub const GRAVITY: f32 = 32.0;

/// Upward speed a jump starts with, in blocks per second. `8.9` clears a
/// one-block step with a little room to spare under [`GRAVITY`]
/// (peak height = speed² / 2g ≈ 1.24 blocks).
pub const JUMP_SPEED: f32 = 8.9;

/// The fastest the player can fall, in blocks per second, however long the
/// fall. Also keeps a long fall from covering so much ground in one frame
/// that collision has to take many small steps to follow it.
pub const TERMINAL_VELOCITY: f32 = 60.0;

/// Width of the player's collision box on both horizontal axes, in blocks.
/// Narrower than the 1-block-wide model (arms included), the same way
/// Minecraft's 0.6 hitbox is narrower than its player, so the player fits
/// through one-block gaps.
pub const HITBOX_WIDTH: f32 = 0.6;

/// Height of the player's collision box, in blocks, from the feet up.
/// Slightly under the model's 2 blocks, so the player fits through two-block
/// gaps without catching on the ceiling.
pub const HITBOX_HEIGHT: f32 = 1.9;

/// Whether hitting a wall cancels a `SprintMode::DoubleTap` sprint that's
/// already engaged, requiring a fresh double-tap to resume it once past the
/// obstacle.
///
/// Requested: running into a block and jumping over it, while still holding
/// the movement key, otherwise keeps the old sprint engaged the whole way —
/// it never got a chance to disengage, since sprint only turns off when
/// every movement key is released. Only affects `DoubleTap`: `Hold` mode
/// reads `SPRINT_HOLD_KEY` fresh every frame regardless, so there's no
/// sticky flag for a collision to reset. See `player::physics::Motion`,
/// which tracks whether the *last* move was stopped on `x`/`z` — never `y`,
/// since landing blocks that axis every frame while walking and must never
/// cancel sprint on its own.
pub const RESET_SPRINT_ON_COLLISION: bool = true;
