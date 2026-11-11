use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    Db,
    models::{CreateTodo, Todo, UpdateTodo},
};

pub async fn health() -> &'static str {
    "OK"
}

pub async fn list_todos(State(db): State<Db>) -> Json<Vec<Todo>> {
    let todos = db.read().unwrap().values().cloned().collect();
    Json(todos)
}

pub async fn create_todo(
    State(db): State<Db>,
    Json(payload): Json<CreateTodo>,
) -> (StatusCode, Json<Todo>) {
    let todo = Todo {
        id: Uuid::new_v4(),
        title: payload.title,
        completed: false,
    };
    db.write().unwrap().insert(todo.id, todo.clone());
    (StatusCode::CREATED, Json(todo))
}

pub async fn get_todo(
    State(db): State<Db>,
    Path(id): Path<Uuid>,
) -> Result<Json<Todo>, StatusCode> {
    db.read()
        .unwrap()
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn update_todo(
    State(db): State<Db>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateTodo>,
) -> Result<Json<Todo>, StatusCode> {
    let mut store = db.write().unwrap();
    let todo = store.get_mut(&id).ok_or(StatusCode::NOT_FOUND)?;

    if let Some(title) = payload.title {
        todo.title = title;
    }
    if let Some(completed) = payload.completed {
        todo.completed = completed;
    }

    Ok(Json(todo.clone()))
}

pub async fn delete_todo(State(db): State<Db>, Path(id): Path<Uuid>) -> StatusCode {
    match db.write().unwrap().remove(&id) {
        Some(_) => StatusCode::NO_CONTENT,
        None => StatusCode::NOT_FOUND,
    }
}
