mod client;
mod core;
mod error;
mod unzip;
mod user;

/// A re-export module, user should only use this function
pub mod prelude {
    pub use crate::core::download;
    pub use crate::user::UserSession;
}
