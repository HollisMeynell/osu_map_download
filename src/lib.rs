mod core;
mod session;

/// A re-export module, user should only use this function
pub mod prelude {
    pub use crate::session::UserSession;
    pub use crate::core::{download, login, visit_home};
}
