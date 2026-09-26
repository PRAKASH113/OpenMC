//! Small, self-contained pieces with no ongoing design questions.
//!
//! Currently just [`log`], which decides which log targets are worth
//! printing. The bar for belonging here is that a module is *complete* — not
//! merely small — so it can hold more later without becoming a place where
//! unrelated things collect by default.

pub mod log;
