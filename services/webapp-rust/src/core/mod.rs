// Templates and web server
use axum::{extract::Multipart, extract::Query, http::StatusCode, response::Json};

// Filesystem operations
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;
use sysinfo::{Disk, DiskExt, System, SystemExt};
use tar::Archive;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use anyhow;

pub mod constants;
use crate::core::constants::{
    ARCHIVES_ROOT_DIR, ARCHIVES_TMP_DIR, ENTRY_POINT_DIR_NAME, VERSIONS_PATH, ZFILL_PADDING,
};

// JSON
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// Data structures
// -----------------------------------------------------------------------------

// Represent the current status of the API
#[derive(Debug, Serialize)]
pub struct StatusData {
    pub uptime: String,
    pub total_memory: String,
    pub used_memory: String,
    pub disk_info: Vec<DiskData>,
}

#[derive(Debug, Serialize)]
pub struct DiskData {
    pub name: String,
    pub available_space: String,
    pub total_space: String,
}

#[derive(Debug, Serialize)]
pub struct ImageData {
    b64: String,
}

#[derive(Debug, Deserialize)]
pub struct ImageQuery {
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionsData {
    last_version: i32,
    last_modified: String,
}

#[derive(Debug, Serialize)]
pub struct InventoryNodeData {
    name: String,
    children: Vec<InventoryNodeData>,
    is_file: bool,
    file_path: String,
}

pub struct ArchiveInfo {
    pub name: String,
    pub version: i32,
}

#[derive(Debug, Serialize)]
pub struct InventoryData {
    root: String,
    children: Vec<InventoryNodeData>,
}

// -----------------------------------------------------------------------------
// Functions
// -----------------------------------------------------------------------------
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

        let file_path = entry_path
            .canonicalize()
            .unwrap_or(PathBuf::from("unknown_path"))
            .display()
            .to_string();

        if entry_path.is_file() {
            nodes_data.push(InventoryNodeData {
                name: file_name_string,
                children: vec![],
                is_file: true,
                file_path,
            });
        }
        // Recurse
        else if entry_path.is_dir() {
            let children = collect_data_from_directory(&entry_path);
            nodes_data.push(InventoryNodeData {
                name: String::from(file_name_string),
                children,
                is_file: false,
                file_path,
            });
        }
    }

    nodes_data
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

        let now: DateTime<Utc> = SystemTime::now().into();
        let last_modified = now.to_rfc3339();

        let initial_data = &VersionsData {
            last_version: 1,
            last_modified,
        };
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
    eprintln!("Running sanitization process..");
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
    let now: DateTime<Utc> = SystemTime::now().into();

    // Write time of last upload
    let last_modified = now.to_rfc3339();
    let new_data = &VersionsData {
        last_version: new_version,
        last_modified,
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

pub fn get_base64_for_path(path: &Path) -> anyhow::Result<String> {
    // TODO: cache all of this

    if !path.exists() {
        let message = format!("Path {} doesn't exist on disk.", path.display());
        anyhow::bail!(message);
    }

    let b64_string;
    eprintln!("Generating base64 of {}", path.display());

    // TODO: make this async
    match fs::read(path) {
        Ok(bytes) => {
            b64_string = base64::encode(bytes);
        }
        Err(e) => {
            let message = format!("Failed to base64 encode {}. Error: {}", path.display(), e);
            anyhow::bail!(message);
        }
    }

    Ok(b64_string)
}

// -----------------------------------------------------------------------------
// API Routes
// -----------------------------------------------------------------------------

// TODO: implement authentication

// Return generic informations about the status of the API
pub async fn status() -> Json<StatusData> {
    // Gather generic info on the system
    let mut sys = System::new_all();
    sys.refresh_all();

    let total_memory = format!(
        "{} bytes",
        bytes_to_human_readable(sys.total_memory() as f64)
    );
    let used_memory = format!(
        "{} bytes",
        bytes_to_human_readable(sys.used_memory() as f64)
    );

    let mut disk_info = Vec::new();
    let mut disks_names_already_added = Vec::new();
    for disk in sys.disks() {
        let disk_name = format!("{}", disk.name().to_str().unwrap_or("unknown"));
        let total_space = bytes_to_human_readable(disk.total_space() as f64);
        let available_space = bytes_to_human_readable(disk.available_space() as f64);

        if disks_names_already_added.contains(&disk_name) {
            continue;
        }

        disk_info.push(DiskData {
            name: disk_name.clone(),
            available_space,
            total_space,
        });
        disks_names_already_added.push(disk_name);
    }

    let mut uptime = String::from("unknown");
    let start_time_as_str = std::env::var("START_TIME").unwrap_or(String::from(""));
    eprintln!("Start time: {}", start_time_as_str);
    let start_time;

    match chrono::DateTime::parse_from_rfc3339(&start_time_as_str) {
        Ok(r) => {
            start_time = r;

            // Convert to UTC to do the math
            let now_utc = chrono::offset::Utc::now();
            let start_time_utc: chrono::DateTime<Utc> = start_time.with_timezone(&Utc);
            let elapsed = now_utc - start_time_utc;

            uptime = format!(
                "{} weeks, {}, days, {} hours, {} minutes, {} seconds",
                elapsed.num_weeks(),
                elapsed.num_days(),
                elapsed.num_hours(),
                elapsed.num_minutes(),
                elapsed.num_seconds()
            );
        }
        Err(e) => {
            eprintln!("Failed to parse datetime from rfc3339: {e}");
        }
    }

    Json(StatusData {
        total_memory,
        used_memory,
        disk_info,
        uptime,
    })
}

// From a base64 that represents the path to the image on disk,
// read the image and return a base64 version of the content of the image.
pub async fn image_preview(query: Query<ImageQuery>) -> Json<ImageData> {
    let base64_path = &query.path;

    let mut image_as_b64 = String::from("");

    let image_path;
    match base64::decode(base64_path) {
        Ok(r) => match std::str::from_utf8(&r) {
            Ok(s) => {
                image_path = PathBuf::from(s);
            }
            Err(e) => {
                eprintln!("Failed to decode UTF-8 text from base64. Error: {}", e);
                return Json(ImageData { b64: image_as_b64 });
            }
        },
        Err(e) => {
            eprintln!("Failed to decode image from input data. Error: {}", e);
            return Json(ImageData { b64: image_as_b64 });
        }
    }

    match get_base64_for_path(&image_path) {
        Ok(result) => {
            image_as_b64 = result;
        }
        Err(_) => {
            return Json(ImageData { b64: image_as_b64 });
        }
    }

    Json(ImageData { b64: image_as_b64 })
}

// List the current images we have on disk
pub async fn list_inventory() -> Json<InventoryData> {
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
    if !archive_path.as_path().exists() {
        let inventory_data = InventoryData {
            root: String::from("root"),
            children: vec![],
        };
        return Json(inventory_data);
    }

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
    if !input_dir.as_path().exists() {
        let inventory_data = InventoryData {
            root: String::from("root"),
            children: vec![],
        };
        return Json(inventory_data);
    }

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
pub async fn upload_archive(mut multipart: Multipart) -> Result<(), (StatusCode, String)> {
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
