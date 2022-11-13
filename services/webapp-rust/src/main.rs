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
use flate2::read::GzDecoder;
use tar::Archive;

// JSON
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
// Filesystem operations
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::{self, io::AsyncReadExt}; // trait needed for write_all()

const PORT_NUM: u16 = 3000;
const APP_VERSION: &'static str = "v0.1.0";
const ENTRY_POINT_DIR_NAME: &'static str = "programm"; // arbitrary name given by vmassimi

const ARCHIVES_ROOT_DIR: &'static str = "/app/data/archives";
const ARCHIVES_TMP_DIR: &'static str = "/app/data/archives/tmp";
const VERSIONS_PATH: &'static str = "/app/data/versions.json";

const ZFILL_PADDING: usize = 3;

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
        .route("/api/upload-archive", post(upload_archive))
        .route("/api/inventory", get(list_inventory))
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
// Data structures
// -----------------------------------------------------------------------------
#[derive(Debug, Serialize)]
struct InventoryNodeData {
    name: String,
    children: Vec<InventoryNodeData>,
}

#[derive(Debug, Serialize)]
struct InventoryData {
    root: String,
    children: Vec<InventoryNodeData>,
}

struct ArchiveInfo {
    name: String,
    version: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct VersionsData {
    last_version: i32,
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

fn find_entry_point_dir(path: &PathBuf) -> Option<PathBuf> {
    let entries;
    match fs::read_dir(&path) {
        Ok(r) => {
            entries = r;
        }
        Err(e) => {
            eprintln!("Failed to read directory {}. Error: {}", path.display(), e);
            return None;
        }
    }

    let mut entry_path;
    let mut entry_name;

    for e in entries {
        match e {
            Ok(entry) => {
                entry_path = entry.path().clone();
                match entry_path.file_name() {
                    Some(r) => {
                        entry_name = r.clone();
                    }
                    None => {
                        continue;
                    }
                }
            }
            Err(_) => {
                continue;
            }
        }

        let file_name = entry_name.to_str().unwrap_or("unknown_name");
        let file_name_string = String::from(file_name);

        if entry_path.is_dir() {
            // Exit condition
            if file_name_string.contains(ENTRY_POINT_DIR_NAME) {
                eprintln!("Found entry point of archive: {file_name_string}");
                return Some(entry_path.canonicalize().unwrap());
            }
            // Recurse
            else {
                let result = find_entry_point_dir(&entry_path);
                match result {
                    Some(_) => {
                        return result;
                    }
                    None => {}
                }
            }
        }
    }

    None
}

fn collect_data_from_directory(path: &PathBuf) -> Vec<InventoryNodeData> {
    let mut nodes_data = Vec::<InventoryNodeData>::new();

    let entries;
    match fs::read_dir(&path) {
        Ok(r) => {
            entries = r;
        }
        Err(e) => {
            eprintln!("Failed to read directory {}. Error: {}", path.display(), e);
            return nodes_data;
        }
    }

    let mut entry_path;
    let mut entry_name;

    for e in entries {
        match e {
            Ok(entry) => {
                entry_path = entry.path().clone();
                match entry_path.file_name() {
                    Some(r) => {
                        entry_name = r.clone();
                    }
                    None => {
                        continue;
                    }
                }
            }
            Err(_) => {
                continue;
            }
        }

        let file_name = entry_name.to_str().unwrap_or("unknown_name");
        let file_name_string = String::from(file_name);

        if entry_path.is_file() {
            nodes_data.push(InventoryNodeData {
                name: file_name_string,
                children: vec![],
            });
        }
        // Recurse
        else if entry_path.is_dir() {
            let children = collect_data_from_directory(&entry_path);
            nodes_data.push(InventoryNodeData {
                name: String::from(file_name_string),
                children,
            });
        }
    }

    nodes_data
}

async fn list_inventory() -> Json<InventoryData> {
    let latest_version;
    match get_archive_version().await {
        Ok(version) => {
            latest_version = version;
        }
        Err(error) => {
            eprintln!("Failed to get archive version. Error: {}", error.1);
            let default_data = InventoryData {
                root: String::from("root"),
                children: vec![],
            };
            return Json(default_data);
        }
    }

    // Look on disk and collect information for all files
    let archive_path = get_archive_path(latest_version);

    // Find the directory that actually contains the root of the archive
    // NB: it has a specific name
    let entry_point_dir = find_entry_point_dir(&archive_path);
    let mut input_dir = archive_path.clone();
    match entry_point_dir {
        Some(r) => {
            input_dir = r;
        }
        None => {}
    }
    eprintln!("Path to input directory: {}", input_dir.display());

    let root_children = collect_data_from_directory(&input_dir);

    let inventory_data = InventoryData {
        root: String::from("root"),
        children: root_children,
    };

    Json(inventory_data)
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
        // FIXME: make this a bit tidier

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

                // TODO: Keep track of versions of the same file

                // TODO: Have all of the following happen in a different Thread?
                let (archive_path, archive_version) = save_archive(data).await?;
                extract_archive(&archive_path, &archive_version).await?;
                update_latest_version().await?;
                clean_up_tmp().await?;
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

// -----------------------------------------------------------------------------
// Various utility functions
// FIXME: move in a lib.rs module
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

async fn save_archive(data: axum::body::Bytes) -> Result<(PathBuf, String), (StatusCode, String)> {
    // Ask the DB which version of the file this is
    let last_version = get_archive_version().await?;

    // Understand where to save
    let version_padded = format!("{:0ZFILL_PADDING$}", last_version + 1);
    let base_dir = Path::new(ARCHIVES_TMP_DIR);
    let save_path = base_dir.join(format!("{}.tar.gz", version_padded));
    match tokio::fs::create_dir_all(base_dir).await {
        Ok(_) => {}
        Err(e) => {
            eprintln!(
                "Failed to create dir: {}. Error: {}",
                save_path.display(),
                e
            );
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }

    eprintln!("Saving file to disk to {}", save_path.display());

    // TODO: Generate a unique checksum?

    // Keep track of elapsed time, for benchmarking reasons
    let current_time = SystemTime::now();

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
            eprintln!("{} written to disk!", save_path.display());
        }
        Err(e) => {
            eprintln!("Failed writing file to disk. Error: {}", e);
            return Err((StatusCode::BAD_REQUEST, e.to_string()));
        }
    }
    match current_time.elapsed() {
        Ok(elapsed) => {
            eprintln!("Saving file to disk took {} seconds", elapsed.as_secs());
        }
        Err(e) => {
            eprintln!("Failed to get elapsed time. Error: {}", e);
        }
    }
    Ok((save_path, version_padded))
}

async fn get_archive_version() -> Result<i32, (StatusCode, String)> {
    // TODO: Have a proper DB, for now a JSON file on disk is enough
    let versions_file_path = Path::new(&VERSIONS_PATH);

    // If we don't have any, write the initial JSON to disk
    if !versions_file_path.exists() {
        let serialized;
        let initial_data = &VersionsData { last_version: 1 };
        match serde_json::to_string_pretty(&initial_data) {
            Ok(r) => {
                serialized = r;
            }
            Err(e) => {
                eprintln!("Failed to serialize {:#?}. Error: {}", initial_data, e);
                return Ok(1);
            }
        }

        match tokio::fs::write(&versions_file_path, serialized).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to create JSON file. Error: {}", e);
            }
        }
    }

    //eprintln!("Reading {}", versions_file_path.display());
    let file_contents;
    match tokio::fs::read_to_string(versions_file_path).await {
        Ok(f) => {
            file_contents = f;
        }
        Err(e) => {
            eprintln!(
                "Failed to read {}, error: {}",
                versions_file_path.display(),
                e
            );
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }

    let data: VersionsData;
    match serde_json::from_str(&file_contents) {
        Ok(r) => {
            data = r;
            return Ok(data.last_version);
        }
        Err(e) => {
            eprintln!("Failed to deserialize {:?}, error: {}", file_contents, e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }
}

async fn extract_archive(archive_path: &Path, version: &str) -> Result<(), (StatusCode, String)> {
    eprintln!("Started decompressing and untaring of archive");
    let tar;

    // Sadly, the 'tar' crate doesn't support async
    match std::fs::File::open(archive_path) {
        Ok(tar_gz) => {
            tar = GzDecoder::new(tar_gz);
        }
        Err(e) => {
            eprintln!("Failed to open tar archive: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }

    let mut archive = Archive::new(tar);
    let extraction_path = Path::new(ARCHIVES_ROOT_DIR).join(&version);
    match archive.unpack(&extraction_path) {
        Ok(()) => {
            println!(
                "Successfully unpacked archive to {}",
                extraction_path.display()
            );
        }
        Err(e) => {
            eprintln!("Failed to extract tar archive: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }

    // Sanitize the names of the directories and files
    // TODO: Check exit code (to catch python tracebacks)
    let output = Command::new("/app/scripts/sanitize_directories.py")
        .args([&extraction_path])
        .output()
        .unwrap();
    eprintln!("Output of sanitization process: {:?}", output.stdout);

    Ok(())
}

async fn update_latest_version() -> Result<(), (StatusCode, String)> {
    eprintln!("Updating versions file to correct the last version..");
    let last_version = get_archive_version().await?;
    let new_version = last_version + 1;
    let versions_file_path = Path::new(&VERSIONS_PATH);

    let serialized_data;
    let new_data = &VersionsData {
        last_version: new_version,
    };
    match serde_json::to_string_pretty(&new_data) {
        Ok(r) => {
            serialized_data = r;
        }
        Err(e) => {
            eprintln!("Failed to serialize {:#?}. Error: {}", new_data, e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }

    match tokio::fs::write(&versions_file_path, serialized_data).await {
        Ok(_) => {
            eprintln!("Last version is now {}", new_version);
        }
        Err(e) => {
            eprintln!("Failed to create JSON file. Error: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }

    Ok(())
}

async fn clean_up_tmp() -> Result<(), (StatusCode, String)> {
    match std::fs::remove_dir_all(ARCHIVES_TMP_DIR) {
        Ok(_) => {
            eprintln!("Successfully cleaned up tmp directory.");
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to clean up tmp directory. Error: {}", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
        }
    }
}

fn get_archive_path(version: i32) -> PathBuf {
    let version_padded = format!("{:0ZFILL_PADDING$}", version);
    let archive_path = Path::new(ARCHIVES_ROOT_DIR).join(version_padded);

    archive_path
}
