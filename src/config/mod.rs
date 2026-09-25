//! Everything configurable, in one place.
//!
//! These are the defaults the game starts with — plain values, grouped by
//! what they configure, so a change can be found and made without reading
//! any of the code that consumes them.
//!
//! Turning values into engine settings is not done here. That belongs to
//! whoever consumes them: [`crate::utils::window`] builds the window from
//! [`window`], and the systems that read input use [`input`] directly.

pub mod input;
pub mod window;
