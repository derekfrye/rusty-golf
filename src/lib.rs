pub mod model;
pub mod controller {
    pub mod db_prefill;
    pub mod espn;
    pub mod score;
}
pub mod view {
    pub mod index;
    pub mod score;
}

pub mod mvu {
    pub mod score;
    pub mod runtime;
}

const HTMX_PATH: &str = "https://unpkg.com/htmx.org@1.9.12";

// // Re-export commonly used items for easier access in tests and other modules
// pub use controller::score::scores;
// pub use db::db::{DatabaseType, Db, DbConfigAndPool};
// pub use model::CacheMap;

// If you have functions like `get_title_from_db`, re-export them as well
pub use model::get_event_details; // Adjust based on actual function location

pub mod args;
