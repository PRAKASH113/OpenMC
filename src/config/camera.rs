//! Camera settings.
//!
//! Currently just field of view — a settings surface like
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
pub const FOV_DEGREES: f32 = 45.0;
