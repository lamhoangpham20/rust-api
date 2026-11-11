pub mod handlers;
pub mod models;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use axum::{Router, routing::get};
use uuid::Uuid;

use models::Todo;

/// Shared, thread-safe in-memory store. Cloning it is cheap (Arc).
pub type Db = Arc<RwLock<HashMap<Uuid, Todo>>>;

/// Builds the router. Kept separate from main() so tests can call it.
pub fn app() -> Router {
    let db: Db = Arc::default();

    Router::new()
        .route("/health", get(handlers::health))
        .route(
            "/todos",
            get(handlers::list_todos).post(handlers::create_todo),
        )
        .route(
            "/todos/{id}",
            get(handlers::get_todo)
                .patch(handlers::update_todo)
                .delete(handlers::delete_todo),
        )
        .with_state(db)
}
