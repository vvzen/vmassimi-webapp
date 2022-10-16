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
use std::path::Path;
use std::time::{Duration, SystemTime};
use tokio;
use tokio::fs::File;
use tokio::io::AsyncWriteExt; // trait needed for write_all()

const PORT_NUM: u16 = 3000;
const APP_VERSION: &'static str = "v0.1.0";
const ENTRY_POINT_DIR_NAME: &'static str = "PROGRAMM"; // arbitrary name given by vmassimi

#[tokio::main]
async fn main() {
    //let address = SocketAddr::from(([0, 0, 0, 1], PORT_NUM));
    let address = format!("0.0.0.0:{}", PORT_NUM);
    println!("Axum server running on {}", address);

    // Create the routes
    let app = Router::new()
        .route("/", get(index))
        .route("/hello/:name", get(hello_name))
        .route("/api/json", get(hello_json))
        .route("/upload-archive", post(upload_archive))
        .route("/inventory", get(inventory));

    // Run the app via hyper
    // axum::Server is a re-export of hyper::Server
    // (https://github.com/hyperium/hyper)
    axum::Server::bind(&address.parse().unwrap())
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

async fn inventory() -> impl IntoResponse {
    // TODO: Read all of the images that we have

    let template = InventoryTemplate {};
    HtmlTemplate(template)
}

// TODO: implement Content-length limit via RequestBodyLimitLayer
// https://docs.rs/axum/latest/axum/extract/struct.ContentLengthLimit.html
// https://github.com/tokio-rs/axum/blob/0.5.x/examples/multipart-form/src/main.rs
async fn upload_archive(mut multipart: Multipart) -> Result<(), (StatusCode, String)> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()))?
    {
        // FIXME: watch training on map_err()

        // Parse the current upload
        match field.name() {
            Some(r) => {
                let name = r.to_string();

                let data;
                match field.bytes().await {
                    Ok(d) => {
                        data = d;
                    }
                    Err(e) => {
                        return Err((StatusCode::BAD_REQUEST, e.to_string()));
                    }
                }

                // Parse content type
                if name == "content-type" {
                    let content_type;
                    match std::str::from_utf8(&data) {
                        Ok(r) => {
                            content_type = r;
                            eprintln!("Content type is {}", content_type);
                        }
                        Err(e) => {
                            eprintln!("Failed to parse field data as UTF8 string: {}", e);
                            return Err((StatusCode::BAD_REQUEST, e.to_string()));
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

                let save_path = Path::new("/app/data/archives").join(&name);
                println!("Saving file to disk to {}", save_path.display());

                // TODO: Keep track of versions of the same file

                // Keep track of elapsed time, for benchmarking reasons
                let current_time = SystemTime::now();

                // Save the data to disk
                let mut file;
                match File::create(&save_path).await {
                    Ok(f) => {
                        file = f;
                    }
                    Err(e) => {
                        eprintln!("Failed to create file to disk. Error: {}", e);
                        return Err((StatusCode::BAD_REQUEST, e.to_string()));
                    }
                }
                match file.write_all(&data).await {
                    Ok(_) => {
                        println!("{} written to disk!", save_path.display());
                    }
                    Err(e) => {
                        eprintln!("Failed writing file to disk. Error: {}", e);
                        return Err((StatusCode::BAD_REQUEST, e.to_string()));
                    }
                }
                match current_time.elapsed() {
                    Ok(elapsed) => {
                        println!("Saving file to disk took {} seconds", elapsed.as_secs());
                    }
                    Err(e) => {
                        eprintln!("Failed to get elapsed time. Error: {}", e);
                    }
                }
            }
            None => {
                return Err((
                    StatusCode::EXPECTATION_FAILED,
                    String::from("No field name in multipart data"),
                ))
            }
        }
    }

    Ok(())
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

#[derive(Template)]
#[template(path = "inventory.html")]
struct InventoryTemplate {}

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

// -----------------------------------------------------------------------------
// Various utility functions
// -----------------------------------------------------------------------------

fn bytes_to_human_readable(num_bytes: f64) -> String {
    // Convert bytes to human-readable values
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
    // Use only the first 3 digit to represent the number, it will be enough
    let result = format!("~{} {}", &file_size_human_readable[0..4], unit_to_use);

    result
}
