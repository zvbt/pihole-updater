use reqwest;
use std::collections::HashSet;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use tokio::task;
use env_logger::Env;
use log::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let file_path = "links.txt";
    let lines = read_lines(file_path)?;

    let mut handles = vec![];

    for line in lines {
        if let Ok(url) = line {
            let handle = task::spawn(async move {
                let url_clone = url.clone(); // Clone the URL for logging
                info!("Downloading and parsing: {}", &url_clone);
                match fetch_and_parse(url).await {
                    Ok(parsed_lines) => {
                        info!("Done downloading and parsing: {}", &url_clone);
                        Ok(parsed_lines)
                    }
                    Err(err) => {
                        info!("Error downloading and parsing {}: {}", &url_clone, err);
                        Err("Fetch and parse error".to_string())
                    }
                }
            });
            handles.push(handle);
        }
    }

    let mut all_lines = HashSet::new();

    for handle in handles {
        match handle.await?? {
            Ok(parsed_lines) => all_lines.extend(parsed_lines),
            Err(err) => {
                eprintln!("Error from task: {}", err);
            }
        }
    }

    let mut sorted_lines: Vec<String> = all_lines.into_iter().collect();
    sorted_lines.sort();

    let output_file_path = "ads_list.txt";
    write_lines_to_file(output_file_path, &sorted_lines)?;

    // Move the file only if the OS is Linux
    #[cfg(target_os = "linux")]
    move_file(output_file_path)?;

    // Log an error if attempting to move file on non-Linux platform
    #[cfg(not(target_os = "linux"))]
    {
        error!("Cannot move file to /var/www/html: Not running on Linux");
    }

    Ok(())
}

async fn fetch_and_parse(url: String) -> Result<Result<HashSet<String>, String>, reqwest::Error> {
    let response = reqwest::get(&url).await?;
    let body = response.text().await?;

    let mut parsed_lines = HashSet::new();
    for line in body.lines() {
        let trimmed_line = line.trim();
        if !trimmed_line.is_empty() && !trimmed_line.starts_with('#') {
            parsed_lines.insert(trimmed_line.to_string());
        }
    }

    Ok(Ok(parsed_lines))
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn write_lines_to_file(filename: &str, lines: &[String]) -> io::Result<()> {
    let mut output_file = File::create(filename)?;
    for line in lines {
        writeln!(output_file, "{}", line)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn move_file(filename: &str) -> io::Result<()> {
    let source = PathBuf::from(filename);
    let target_dir = "/var/www/html";

    // Check if the target directory exists, create it if necessary
    if !Path::new(target_dir).exists() {
        fs::create_dir_all(target_dir)?;
    }

    let target = PathBuf::from(format!("{}/{}", target_dir, source.file_name().unwrap().to_str().unwrap()));
    fs::rename(source, target)?;

    Ok(())
}