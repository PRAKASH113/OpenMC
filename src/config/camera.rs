//! Camera settings.
//!
//! Field of view and the third-person framing — a settings surface like
//! [`crate::config::input`], meant to be read and tuned directly rather than
//! reasoned about.

/// Vertical field of view, in degrees.
///
/// Bevy's own default for a perspective camera is 45°, kept here as the
/// starting value rather than silently inherited — see
/// [`crate::camera::camera_world`] for where it is applied. Raise it for a
/// wider view; a common "FPS" feel sits around 90-110°, though higher values
/// flatten depth cues and distort more toward the edges of the frame. Lower
/// it for something closer to a telephoto lens.
pub const FOV_DEGREES: f32 = 60.0;

/// How far the third-person camera sits back from the point it orbits, in
/// blocks. Minecraft's own third-person view uses 4.
pub const THIRD_PERSON_DISTANCE: f32 = 4.0;

/// Height above the player's feet that the camera orbits around and looks at,
/// in blocks. Roughly eye level on the 2-block-tall player model, so the view
/// centres on the head rather than the feet.
pub const THIRD_PERSON_PIVOT_HEIGHT: f32 = 1.6;
