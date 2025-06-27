//! Commands module

pub mod compile;
pub mod executable;
pub mod run;
pub mod check;

pub use compile::compile;
pub use run::run;
pub use check::check;