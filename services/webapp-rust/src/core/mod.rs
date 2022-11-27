// Templates and web server
use axum::{extract::Multipart, extract::Query, http::StatusCode, response::Json};

// Filesystem operations
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use std::convert::TryInto;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;
use std::time::SystemTime;
use sysinfo::{Disk, DiskExt, System, SystemExt};
use tar::Archive;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

// JSON
use serde::{Deserialize, Serialize};

use uuid::Uuid;

// Errors
use anyhow;

pub mod constants;
use crate::core::constants::{
    ARCHIVES_ROOT_DIR, ARCHIVES_TMP_DIR, ENTRY_POINT_DIR_NAME, JOBS_ROOT_DIR, VERSIONS_PATH,
    ZFILL_PADDING,
};

// -----------------------------------------------------------------------------
// Data structures
// -----------------------------------------------------------------------------

pub struct Page {
    pub active: bool,
    pub name: String,
    pub url: String,
}

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

#[derive(Debug, Serialize)]
pub enum JobStatus {
    STARTED,
    FAILED,
    COMPLETED,
    UNKNOWN,
}

#[derive(Debug, Serialize)]
pub struct JobData {
    endpoint: String,
    job_id: String,
    status: JobStatus,
    image: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct JobQuery {
    pub job_id: String,
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

async fn save_archive(data: axum::body::Bytes) -> anyhow::Result<(PathBuf, String)> {
    // Ask the DB which version of the file this is
    let last_version = get_archive_version().await?;

    // Understand where to save
    let version_padded = format!("{:0ZFILL_PADDING$}", last_version + 1);
    let base_dir = Path::new(ARCHIVES_TMP_DIR);
    let save_path = base_dir.join(format!("{}.tar.gz", version_padded));
    match tokio::fs::create_dir_all(base_dir).await {
        Ok(_) => {}
        Err(e) => {
            let message = format!(
                "Failed to create dir: {}. Error: {}",
                save_path.display(),
                e
            );
            anyhow::bail!(message);
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
            let message = format!("Failed to create file to disk. Error: {}", e);
            anyhow::bail!(message);
        }
    }
    match file.write_all(&data).await {
        Ok(_) => {
            eprintln!("{} written to disk!", save_path.display());
        }
        Err(e) => {
            let message = format!("Failed writing file to disk. Error: {}", e);
            anyhow::bail!(message);
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

pub async fn get_archive_version() -> anyhow::Result<i32> {
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
            let message = format!(
                "Failed to read {}, error: {}",
                versions_file_path.display(),
                e
            );
            anyhow::bail!(message);
        }
    }

    let data: VersionsData;
    match serde_json::from_str(&file_contents) {
        Ok(r) => {
            data = r;
            return Ok(data.last_version);
        }
        Err(e) => {
            let message = format!("Failed to deserialize {:?}, error: {}", file_contents, e);
            anyhow::bail!(message);
        }
    }
}

async fn extract_archive(archive_path: &Path, version: &str) -> anyhow::Result<()> {
    eprintln!("Started decompressing and untaring of archive");
    let tar;

    // Sadly, the 'tar' crate doesn't support async
    match std::fs::File::open(archive_path) {
        Ok(tar_gz) => {
            tar = GzDecoder::new(tar_gz);
        }
        Err(e) => {
            let message = format!("Failed to open tar archive: {}", e);
            anyhow::bail!(message);
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
            let message = format!("Failed to extract tar archive: {}", e);
            anyhow::bail!(message);
        }
    }

    // Sanitize the names of the directories and files
    // TODO: Check exit code (to catch python tracebacks)
    eprintln!("Running sanitization process..");
    let output = Command::new("/app/scripts/sanitize_directories.py")
        .args([&extraction_path])
        .output()
        .unwrap();
    eprintln!("Exit code of sanitization process: {:?}", output.status);
    eprintln!(
        "STDOUT of sanitization process: {:?}",
        std::str::from_utf8(&output.stdout)
    );
    eprintln!(
        "STERR of sanitization process: {:?}",
        std::str::from_utf8(&output.stderr)
    );

    Ok(())
}

async fn update_latest_version() -> anyhow::Result<()> {
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
            let message = format!("Failed to serialize {:#?}. Error: {}", new_data, e);
            anyhow::bail!(message);
        }
    }

    match tokio::fs::write(&versions_file_path, serialized_data).await {
        Ok(_) => {
            eprintln!("Last version is now {}", new_version);
        }
        Err(e) => {
            let message = format!("Failed to create JSON file. Error: {}", e);
            anyhow::bail!(message);
        }
    }

    Ok(())
}

async fn clean_up_tmp() -> anyhow::Result<()> {
    match std::fs::remove_dir_all(ARCHIVES_TMP_DIR) {
        Ok(_) => {
            eprintln!("Successfully cleaned up tmp directory.");
            Ok(())
        }
        Err(e) => {
            let message = format!("Failed to clean up tmp directory. Error: {}", e);
            anyhow::bail!(message);
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

pub fn get_pages_lists_for_current_page(active_page: &str) -> Vec<Page> {
    // Generate urls, titles and understand whether a page should be marked as "active"
    // This information is needed to inform the fixed navbar

    // TODO: use an Enum
    let all_pages = vec!["Upload", "Inventory", "Random"];
    let mut pages = vec![];

    for page_name in all_pages {
        let is_active = page_name == active_page;
        let page_url = String::from(format!("/app/{}", page_name.to_lowercase()));
        let current_page = Page {
            name: String::from(page_name),
            active: is_active,
            url: page_url,
        };
        pages.push(current_page);
    }

    pages
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

    // TODO: implement a mechanism to understand whether a job has been queued or not

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
            eprintln!("Failed to get archive version. Error: {}", error);
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

pub async fn queue_generation_of_random_image() -> Json<JobData> {
    // Generate a random ID
    let job_id = Uuid::new_v4();
    let job_id_str = job_id.to_string();

    eprintln!("Generated new Job, id: {}", job_id_str);
    let url = String::from("api/jobs");

    let job_data = JobData {
        endpoint: url,
        job_id: String::from(&job_id_str),
        status: JobStatus::STARTED,
        image: None,
    };

    // In the background, start the generation of the image
    tokio::spawn(async move {
        eprintln!("Started image processing..");
        match generate_random_image(&job_id_str).await {
            Ok(_) => {
                eprintln!("Finished image processing.");
            }
            Err(e) => {
                eprintln!("Failed to render image. {}", e);
            }
        }
    });

    Json(job_data)
}

fn get_job_path(job_id_str: &str) -> anyhow::Result<PathBuf> {
    let jobs_root_dir = PathBuf::from(JOBS_ROOT_DIR);
    if !jobs_root_dir.exists() {
        match fs::create_dir(&jobs_root_dir) {
            Ok(_) => {}
            Err(e) => {
                anyhow::bail!(e);
            }
        }
    }

    let job_path = PathBuf::from(jobs_root_dir.join(&job_id_str));
    Ok(job_path)
}

// TODO: Proper error handling and return codes
pub async fn get_job(query: Query<JobQuery>) -> Json<JobData> {
    let job_id = &query.job_id;
    eprintln!("Checking for job_id={}", job_id);

    // TODO: Maybe we can save a job file with the .png extension,
    // So that we can check if /path/to/job_id.png exists, and return it,
    // and if it doesn't, then /path/to/job_id will contain the progress %

    let mut job_data = JobData {
        endpoint: String::from("api/jobs"),
        job_id: String::from(job_id),
        status: JobStatus::UNKNOWN,
        image: None,
    };

    // TODO: Distinguish between failed jobs and jobs that don't exist at all
    let failed_data = JobData {
        endpoint: String::from("api/jobs"),
        job_id: String::from(job_id),
        status: JobStatus::FAILED,
        image: None,
    };

    // Check if the job has finished
    // If so, retrieve the related image
    // If not, give a meaningful reply to the caller
    let job_path;
    match get_job_path(job_id) {
        Ok(r) => {
            job_path = r;
        }
        Err(e) => {
            eprintln!("{e}");
            return Json(failed_data);
        }
    }

    if !job_path.exists() {
        eprintln!("{} doesn't exist.", job_path.display());
        return Json(failed_data);
    }

    // If file size is small, it's definitely not a rendered image
    let file_size = &job_path.metadata().unwrap().len();
    eprintln!("file_size: {}", file_size);
    if file_size < &20 {
        // TODO: Extract the progress
        job_data.status = JobStatus::STARTED;
        match fs::read_to_string(&job_path) {
            Ok(content) => {
                eprintln!("Progress: {}", content);
                return Json(job_data);
            }
            Err(e) => {
                eprintln!("Error while reading image content: {e}");
                return Json(failed_data);
            }
        }
    }

    // Image has finished rendering, ideally
    eprintln!("Getting base64 from image..");
    match get_base64_for_path(&job_path) {
        Ok(base64_str) => {
            eprintln!("Image has finished rendering");
            job_data.image = Some(base64_str);
            job_data.status = JobStatus::COMPLETED;
        }
        Err(e) => {
            eprintln!("Error while reading image content: {e}");
            return Json(failed_data);
        }
    }

    Json(job_data)
}

pub async fn generate_random_image(job_id_str: &str) -> anyhow::Result<()> {
    // Write the job file on disk so that we know this request has started
    let job_path = get_job_path(job_id_str)?;

    // Update progress
    match fs::write(&job_path, "progress: 0%") {
        Ok(_) => {}
        Err(e) => {
            let message = format!(
                "Failed to write jobs file ({}) on disk. {}",
                job_path.display(),
                e
            );
            anyhow::bail!(message);
        }
    }

    // First, generate a random recipe
    let latest_archive_version = get_archive_version().await?;

    // Look on disk and collect information for all files
    let archive_path = get_archive_path(latest_archive_version);
    let entry_point_path = archive_path
        .join("sphynx_program")
        .join(ENTRY_POINT_DIR_NAME);

    // TODO: Check exit code (to catch python tracebacks)

    // $ generate_permutation.py /app/data/archives/002 > my_recipe_file
    // $ cat my_recipe_file | ./image-composite/target/release/image-composite --image-name my_name
    eprintln!(
        "Generating permutation starting from {}",
        entry_point_path.display()
    );

    let mut generate = tokio::process::Command::new("/app/scripts/generate_permutation.py")
        .args([&entry_point_path])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn generate_permutation.py");

    // Update progress
    match fs::write(&job_path, "progress: 20%") {
        Ok(_) => {}
        Err(e) => {
            let message = format!(
                "Failed to write jobs file ({}) on disk : {}",
                job_path.display(),
                e
            );
            anyhow::bail!(message);
        }
    }

    let render_stdin: Stdio = generate
        .stdout
        .take()
        .unwrap()
        .try_into()
        .expect("Failed to convert stdout to Stdio");

    let image_name = format!("{}", job_id_str);
    let render = tokio::process::Command::new("/app/image-composite-linux")
        .args(["--image-name", &image_name])
        .stdin(render_stdin)
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn image-composite");

    // TODO: handle STDERR too
    let (generate_result, render_output) = tokio::join!(generate.wait(), render.wait_with_output());

    let generation_has_succeeded = generate_result.unwrap().success();
    eprintln!("Generation has succeded? {}", generation_has_succeeded);

    if !generation_has_succeeded {
        match fs::remove_file(&job_path) {
            Ok(_) => {}
            Err(e) => {
                let message = format!(
                    "Failed to remove jobs file ({}) on disk. {}",
                    job_path.display(),
                    e
                );
                anyhow::bail!(message);
            }
        }
        let message = format!("Image generation has failed.");
        anyhow::bail!(message);
    }

    let render_stdout = &render_output.unwrap().stdout;
    let image_path_str = std::str::from_utf8(render_stdout)
        .unwrap_or("")
        .strip_suffix("\n")
        .unwrap_or("");

    eprintln!("image_path_str: {}", image_path_str);

    let image_path = PathBuf::from(image_path_str);

    if image_path_str.is_empty() {
        anyhow::bail!("No STDOUT generated from render process");
    }
    if !image_path.exists() {
        let message = format!("Image at path {} doesn't exist.", image_path.display());
        anyhow::bail!(message);
    }

    match fs::rename(&image_path, &job_path) {
        Ok(()) => {}
        Err(e) => {
            let message = format!(
                "Failed to move image from {} to {}. {}",
                image_path.display(),
                job_path.display(),
                e
            );
            anyhow::bail!(message);
        }
    }

    Ok(())
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

                // Spawn a different thread to do all of the data cleanup
                // FIXME: tidy up error handling
                tokio::spawn(async move {
                    let (archive_path, archive_version) =
                        save_archive(data).await.expect("Failed to save archive.");

                    match extract_archive(&archive_path, &archive_version).await {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Failed to extract archive. {}", e);
                        }
                    }
                    match update_latest_version().await {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Failed to update latest version. {}", e);
                        }
                    }
                    match clean_up_tmp().await {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("Failed to clean up tmp dir. {}", e);
                        }
                    }
                });
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
