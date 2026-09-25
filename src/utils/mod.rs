//! Small, self-contained pieces that adapt our settings to the engine.
//!
//! Each module here does one finished job with no ongoing design questions:
//! [`window`] creates the window and handles its runtime toggles, [`log`]
//! decides which log targets are worth printing. They are separated from
//! [`crate::app`] so that folder holds only the parts that shape how the
//! game is assembled.
//!
//! The bar for belonging here is that a module is *complete* — not merely
//! small. Anything still growing a design belongs with the domain it serves.

pub mod log;
pub mod window;
