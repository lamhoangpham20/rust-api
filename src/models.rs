use serde::{Deserialize, Serialize};
use uuid::Uuid;

// A todo item as stored and returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: Uuid,
    pub title: String,
    pub completed: bool,
}

// Request body for POST /todos.
#[derive(Debug, Deserialize)]
pub struct CreateTodo {
    pub title: String,
}

// Request body for PATCH /todos/:id. Every field is optional.
#[derive(Debug, Deserialize)]
pub struct UpdateTodo {
    pub title: Option<String>,
    pub completed: Option<bool>,
}
