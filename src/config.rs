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
    pub const MOVE_SPEED: f32 = 8.0;
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
