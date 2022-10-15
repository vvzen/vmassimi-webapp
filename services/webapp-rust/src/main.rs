use axum::{body::Body, response::Json, routing::get, Router};
use serde_json::{json, Value};
use tokio;

const PORT_NUM: i32 = 3000;

#[tokio::main]
async fn main() {
    let address = format!("0.0.0.0:{}", PORT_NUM);
    println!("Axum server running on {}", address);

    // Create the routes
    let app = Router::new()
        .route("/", get(hello_world))
        .route("/json", get(hello_json));

    // Run the app via hyper
    axum::Server::bind(&address.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn hello_world() -> &'static str {
    println!("Serving plain text..");
    "Hello, World!"
}

async fn hello_json() -> Json<Value> {
    println!("Serving JSON..");
    Json(json!({ "the_answer": 42}))
}

async fn upload_file() {}
