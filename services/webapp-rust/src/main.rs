use askama::Template;
use axum::{
    body::Body,
    extract,
    extract::{DefaultBodyLimit, Multipart},
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde_json::{json, Value};
use std::fs;
use std::net::SocketAddr;
use tokio;

const PORT_NUM: u16 = 3000;
const APP_VERSION: &'static str = "v0.1.0";

#[tokio::main]
async fn main() {
    let address = SocketAddr::from(([127, 0, 0, 1], PORT_NUM));
    println!("Axum server running on {}", address);

    // Create the routes
    let app = Router::new()
        .route("/", get(index))
        .route("/hello/:name", get(hello_name))
        .route("/json", get(hello_json))
        .route("/upload", post(upload_file));

    // Run the app via hyper
    // axum::Server is a re-export of hyper::Server
    // (https://github.com/hyperium/hyper)
    axum::Server::bind(&address)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// -----------------------------------------------------------------------------
// Routes
// -----------------------------------------------------------------------------

// The root of the app
async fn index() -> impl IntoResponse {
    let template = IndexTemplate {
        app_version: APP_VERSION,
    };
    HtmlTemplate(template)
}

// A sample route returning plain text
async fn hello_world() -> &'static str {
    println!("Serving plain text..");
    "Hello, World!"
}

// A sample route returning JSON
async fn hello_json() -> Json<Value> {
    println!("Serving JSON..");
    Json(json!({ "the_answer": 42}))
}

// A sample route returning a rendered template
async fn hello_name(extract::Path(name): extract::Path<String>) -> impl IntoResponse {
    let template = HelloTemplate { name };
    HtmlTemplate(template)
}

// TODO: implement Content-length limit via RequestBodyLimitLayer
// https://docs.rs/axum/latest/axum/extract/struct.ContentLengthLimit.html
// https://github.com/tokio-rs/axum/blob/0.5.x/examples/multipart-form/src/main.rs
async fn upload_file(mut multipart: Multipart) {
    while let Some(field) = multipart.next_field().await.unwrap() {
        // Parse the current upload
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        if name == "content-type" {
            let content_type;
            match std::str::from_utf8(&data) {
                Ok(r) => {
                    content_type = r;
                    eprintln!("Content type is {}", content_type);
                }
                Err(e) => {
                    eprintln!("Failed to parse field data as UTF8 string: {}", e);
                }
            }
            continue;
        }
        if !name.contains(".tar.gz") {
            eprintln!("Skipping file since it's not a .tar.gz archive");
            continue;
        }

        let human_readable_size = bytes_to_human_readable(data.len() as f64);
        println!("Length of '{}' is {} bytes", &name, &human_readable_size);

        match fs::write(&name, &data) {
            Ok(()) => {
                println!("File written to disk");
            }
            Err(e) => {
                eprintln!("Failed writing file to disk. Error: {}", e);
            }
        }
    }
}

// -----------------------------------------------------------------------------
// Templating
// -----------------------------------------------------------------------------

// Generic struct for templates
struct HtmlTemplate<T>(T);

// Our custom structs that will be used to render our html templates
#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate {
    name: String,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    app_version: &'static str,
}

// Implement the functionality required to render Generic Askama templates
// into our own HtmlTemplates to be served back from our server
impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", e),
            )
                .into_response(),
        }
    }
}

// Convert bytes to human-readable values
fn bytes_to_human_readable(num_bytes: f64) -> String {
    // This function might not be perfect and very optimized, but at least I wrote it myself!

    // Since this is for humans, I'm not using bi-bytes (which use 1024 as base)
    let base: f64 = 1000.0;
    const UNITS: [&'static str; 9] = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];

    // Understand what unit to use
    let mut exponent = num_bytes.log(base).floor() as i64;
    // Just in case the number is horribly big, clamp it down to a minimum
    exponent = std::cmp::min(exponent, (UNITS.len() - 1) as i64);
    let unit_to_use = UNITS[exponent as usize];

    let file_size_in_unit = num_bytes / base.powf(exponent as f64);
    let file_size_human_readable = file_size_in_unit.to_string();
    let result = format!("~{} {}", &file_size_human_readable[0..4], unit_to_use);

    result
}
