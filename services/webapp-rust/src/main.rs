// Templates and web server
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

// JSON
use serde_json::{json, Value};
// Filesystem operations
use tokio::{self, io::AsyncReadExt}; // trait needed for write_all()

use crate::core::constants::{APP_VERSION, PORT_NUM};
use crate::core::ArchiveInfo;

mod core;

#[tokio::main]
async fn main() {
    //let address = SocketAddr::from(([0, 0, 0, 1], PORT_NUM));
    let address = format!("0.0.0.0:{}", PORT_NUM);
    println!("Axum server running on {}", address);

    // Create the routes
    let app = Router::new()
        .route("/", get(upload))
        .route("/hello/:name", get(hello_name))
        .route("/api/json", get(hello_json))
        .route("/upload", get(upload))
        .route("/api/upload-archive", post(core::upload_archive))
        .route("/api/inventory", get(core::list_inventory))
        .route("/api/image", get(core::image_preview))
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

// The entry point of the app
async fn upload() -> impl IntoResponse {
    let pages = vec![
        Page {
            name: String::from("Upload"),
            active: true,
            url: String::from("/app/upload"),
        },
        Page {
            name: String::from("Inventory"),
            active: false,
            url: String::from("/app/inventory"),
        },
    ];
    let template = UploadTemplate {
        app_version: APP_VERSION,
        title: String::from("Upload"),
        pages,
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
    // TODO: Create a list of all the archives that were uploaded
    // from there, the user should be able to navigate the uploaded images
    // Then, for every uploaded image, he should be able to upload a new version
    let pages = vec![
        Page {
            name: String::from("Upload"),
            active: false,
            url: String::from("/app/upload"),
        },
        Page {
            name: String::from("Inventory"),
            active: true,
            url: String::from("/app/inventory"),
        },
    ];

    // FIXME: This is fake data, it should come from a DB
    let archives = vec![
        ArchiveInfo {
            name: String::from("Sphinx"),
            version: 1,
        },
        ArchiveInfo {
            name: String::from("Sphinx"),
            version: 2,
        },
    ];

    let template = InventoryTemplate {
        title: String::from("Inventory"),
        pages,
        archives,
    };
    HtmlTemplate(template)
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
#[template(path = "upload.html")]
struct UploadTemplate {
    app_version: &'static str,
    title: String,
    pages: Vec<Page>,
}

struct Page {
    active: bool,
    name: String,
    url: String,
}

#[derive(Template)]
#[template(path = "inventory.html")]
struct InventoryTemplate {
    title: String,
    pages: Vec<Page>,
    archives: Vec<ArchiveInfo>,
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
