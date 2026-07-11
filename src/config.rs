pub mod window {
    pub const BORDERLESS_MODE: bool = false;
    pub const WINDOW_TITLE: &str = "OpenMC - Alpha Version";
}

pub mod camera {
    pub const FOV_DEGREES: f32 = 80.0;
}

/// All keyboard and mouse control configuration lives here, in one place.
pub mod controls {
    use bevy::prelude::{KeyCode, Resource};

    #[derive(Resource, Debug, Clone)]
    pub struct KeyBinds {
        pub forward: KeyCode,
        pub backward: KeyCode,
        pub left: KeyCode,
        pub right: KeyCode,
        pub up: KeyCode,
        pub down: KeyCode,
    }

    impl Default for KeyBinds {
        fn default() -> Self {
            Self {
                forward: KeyCode::KeyW,
                backward: KeyCode::KeyS,
                left: KeyCode::KeyA,
                right: KeyCode::KeyD,
                up: KeyCode::Space,
                down: KeyCode::ShiftLeft,
            }
        }
    }

    pub const MOUSE_SENSITIVITY: f32 = 0.002;
    /// `ControlMode::Dev`'s free-fly speed. Deliberately kept fast — every
    /// terrain/lighting/performance round so far relied on flying around
    /// quickly to inspect things, and this mode doesn't try to feel
    /// realistic. `ControlMode::Player`'s on-foot speed is a separate
    /// constant (`config::player::WALK_SPEED`) specifically so slowing down
    /// walking doesn't also slow down dev flying.
    pub const MOVE_SPEED: f32 = 8.0;
}

/// Player collision box and physics tuning for `ControlMode::Player` (see
/// `docs/player-physics.md`). `ControlMode::Dev` (free-fly) ignores all of
/// this entirely.
pub mod player {
    /// Horizontal size of the collision box (X/Z). Not requested explicitly,
    /// so picked to match a typical humanoid footprint at this world's block
    /// scale (roughly Minecraft's own 0.6-wide player box).
    pub const WIDTH: f32 = 0.6;
    /// Vertical size of the collision box — two blocks tall, as requested.
    pub const HEIGHT: f32 = 2.0;
    pub const DEPTH: f32 = 0.6;

    /// How far the camera sits above the collision box's feet — near the top
    /// of the box (like real eyes near the top of a body), not dead center.
    pub const EYE_HEIGHT: f32 = 1.7;

    /// On-foot horizontal speed — separate from `controls::MOVE_SPEED`
    /// (`ControlMode::Dev`'s free-fly speed) on purpose, see that constant's
    /// comment. Lower than the original shared 8.0, which felt too fast for
    /// walking.
    pub const WALK_SPEED: f32 = 5.0;

    /// Downward acceleration in blocks/s². Matches Minecraft's actual gravity
    /// constant (0.08 blocks/tick² at 20 ticks/s = 32 blocks/s²) rather than
    /// real-world 9.8 — at this block scale, real gravity reads as floaty.
    pub const GRAVITY: f32 = 32.0;
    /// Initial upward velocity on jump. `v = sqrt(2 * GRAVITY * height)` —
    /// 8.4 clears about 1.1 blocks, "just a bit over one block" as requested
    /// (9.0's ~1.27 blocks read as too high).
    pub const JUMP_VELOCITY: f32 = 8.4;
    /// Caps fall speed so a long drop can't build up enough per-frame
    /// movement to tunnel through a 1-block-thick floor at normal framerates.
    pub const TERMINAL_VELOCITY: f32 = 40.0;
}

pub mod world {
    pub const CHUNK_SIZE: i32 = 32;

    /// How many chunks (horizontally, around the player) get rendered.
    /// Raised back up from the single-chunk test in
    /// `docs/performance-investigation.md` now that one chunk's stats looked
    /// healthy — a 9x9 grid of chunk columns around the player.
    pub const RENDER_DISTANCE: i32 = 4;

    /// How many chunks (horizontally) get simulated (ticked) — reserved for
    /// later, not used yet. Must be >= RENDER_DISTANCE: you can't render a
    /// chunk that isn't even being simulated.
    pub const SIMULATION_DISTANCE: i32 = 4;

    const _: () = assert!(
        SIMULATION_DISTANCE >= RENDER_DISTANCE,
        "simulation_distance must be >= render_distance"
    );
}
