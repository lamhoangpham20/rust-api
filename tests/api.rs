use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt; // for `oneshot` method

#[tokio::test]
async fn health_returns_ok() {
    let app = rust_api::app();

    let res = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn create_then_fetch_todo() {
    let app = rust_api::app();

    let res = app
        .clone()
        .oneshot(
            Request::post("/todos")
                .header("content-type", "application/json")
                .body(Body::from(json!({"title": "Write tests"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::CREATED);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let created: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(created["title"], "Write tests");
    assert_eq!(created["completed"], false);

    //Fetch it back by id
    let id = created["id"].as_str().unwrap();
    let res = app
        .oneshot(
            Request::get(format!("/todos/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn unknown_todo_returns_404() {
    let app = rust_api::app();
    let res = app
        .oneshot(
            Request::get("/todos/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
