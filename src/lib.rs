mod core;
mod user;
mod error;

/// A re-export module, user should only use this function
pub mod prelude {
    pub use crate::user::UserSession;
    pub use crate::core::download;
}
