pub mod model;
pub mod admin {
    pub mod view {
        pub mod admin00_landing;
        pub mod admin01_tables;
        pub mod admin0x;
    }
    pub mod model {
        pub mod admin_model;
    }
    pub mod router;
}
pub mod controller {
    pub mod cache;
    pub mod espn;
    pub mod score;
}
pub mod view {
    pub mod index;
    pub mod score;
}

const HTMX_PATH: &str = "https://unpkg.com/htmx.org@1.9.12";

// // Re-export commonly used items for easier access in tests and other modules
// pub use controller::score::scores;
// pub use db::db::{DatabaseType, Db, DbConfigAndPool};
// pub use model::CacheMap;

// If you have functions like `get_title_from_db`, re-export them as well
pub use model::get_title_from_db; // Adjust based on actual function location
