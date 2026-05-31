use std::{collections::HashMap, fs::{self, File, read_to_string}, io::{Read, Write}, path::Path};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::{process::Command};

use anyhow::Result;

use crate::{ALLOW_PARTIAL_DOWNLOADS, common, movie::download_with_idec};

pub async fn download_loop(idec: &String, name: &String, total_duration: u64, m: MultiProgress, client: reqwest::Client) -> Result<(), Box<dyn std::error::Error>> {
    let mut retry_count = 0;
    let max_retry_count = 10;

    let pb = m.add(ProgressBar::new(total_duration));
    pb.set_style(
        ProgressStyle::default_bar()
            .template(&"{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>6}/{len:6} s, ETA {eta_precise}: {NAME} - {msg}".replace("{NAME}", name))
            .unwrap()
            .progress_chars("#>-"),
    );

    let playlist_url = common::get_playlist_url(&idec);
    let playlist = common::download_playlist(&playlist_url, client.clone()).await?;

    let mut subtitle_files = HashMap::<String, String>::new();

    if let Ok(subtitle_urls) = common::extract_subtitle_urls(&playlist) {
        pb.set_message(format!("Found {} subtitles.", subtitle_files.len()));

        for (language, subtitle_url) in subtitle_urls {
            let subtitle_filename = common::get_subtitle_filename(&format!("{} - {}", &name, language));
            pb.set_message(format!("Downloading subtitles to '{}'.", subtitle_filename));
            common::download_subtitle(&subtitle_url, &subtitle_filename, client.clone()).await?;

            subtitle_files.insert(language, subtitle_filename);
        }
    } else {
        pb.set_message("No subtitles found.");
    }

    let subtitle_arguments = common::create_subtitle_arguments(&subtitle_files);

    let output_filename = crate::common::get_final_output_filename(name.as_str());

    loop {
        match resume_download(idec, name, total_duration, pb.clone(), client.clone(), retry_count, subtitle_arguments.clone()).await {
            Ok(_) => break,
            Err(e) => {
                pb.set_message(format!("Error during download: {}. Retrying...", e));
                retry_count += 1;
                if retry_count >= max_retry_count {
                    pb.abandon_with_message(format!("Failed to download after {} attempts: {}", max_retry_count, e));
                    return Err(format!("Failed to download after {} attempts: {}", max_retry_count, e).into());
                }
            }
        }
    }

    if ALLOW_PARTIAL_DOWNLOADS {
        join_downloaded_files(name, &output_filename, subtitle_arguments).await?;
    }

    Ok(())
}

pub async fn resume_download(idec: &String, name: &String, total_duration: u64, pb: ProgressBar, client: reqwest::Client, attempt: u32, subtitle_arguments: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let join_file = format!("{}.join", name);

    //println!("Checking for existing download progress for '{}'.", join_file);

    if let Ok(v) = fs::exists(&join_file) && v && ALLOW_PARTIAL_DOWNLOADS {   
        let filenames = sanitize_and_read_join_file(&name)?;
        let durations = get_durations(filenames).await?;
        
        let max_duration = *durations.values().max().unwrap_or(&0);

        //println!("Resuming download for '{}'. Already downloaded {} seconds out of {} seconds.", name, max_duration, total_duration);
        
        download_with_idec(idec, name, total_duration, pb, client, max_duration, attempt, subtitle_arguments).await
    } else {
        download_with_idec(idec, name, total_duration, pb, client, 0, attempt, subtitle_arguments).await
    }
}

pub async fn join_downloaded_files(name: &String, output_filename: &String, subtitle_arguments: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let join_file = format!("{}.join", name);
    join_files(&name, output_filename, subtitle_arguments).await?;
    
    let filenames = sanitize_and_read_join_file(&name)?;

    for filename in filenames {
        //std::fs::remove_file(&filename)?;
        std::fs::remove_file(format!("{}.snapshot", &filename))?;
    }

    std::fs::remove_file(join_file)?;
    Ok(())
}

pub async fn read_snapshot_file(filename: &str) -> Result<u64, Box<dyn std::error::Error>> {
    //println!("Reading snapshot file for '{}'.", filename);

    let snapshot_filename = format!("{}.snapshot", filename);

    if !fs::exists(&snapshot_filename)? {
        //println!("No snapshot file found for '{}'. Starting from the beginning.", filename);
        return Ok(0);
    }

    let mut file = std::fs::File::open(snapshot_filename)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let elapsed_time_us = content.trim().parse::<u64>()?;
    Ok(elapsed_time_us)
}

pub async fn get_durations(filenames: Vec<String>) -> Result<HashMap<String, u64>, Box<dyn std::error::Error>> {
    let mut duration_map = HashMap::new();

    for filename in filenames {
        let duration = read_snapshot_file(&filename).await?;
        duration_map.insert(filename, duration);
    }

    Ok(duration_map)
}

/* pub async fn read_join_file(name: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(format!("{}.join", name))?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let filenames = content.lines()
        .filter_map(|line| line.strip_prefix("file '").and_then(|s| s.strip_suffix("'")))
        .map(|s| s.to_string())
        .collect();

    Ok(filenames)
} */

pub fn sanitize_and_read_join_file(name: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let content = read_to_string(format!("{}.join", name)).expect("Could not read original file");
    let mut clean_lines = Vec::new();

    for line in content.lines() {
        // Look for lines structured like: file 'part1.mkv'
        if line.starts_with("file ") {
            // Strip out the "file " prefix and any surrounding single quotes
            let path_str = line.replace("file ", "").replace('\'', "").trim().to_string();
            
            if Path::new(&path_str).exists() {
                clean_lines.push(line.to_string());
            }
        } else {
            // Keep comments or empty spacing lines intact
            clean_lines.push(line.to_string());
        }
    }

    let mut clean_file = File::create(format!("{}.join", name)).expect("Failed to create sanitized file");
    clean_file.write_all(clean_lines.join("\n").as_bytes()).expect("Write failed");

    let filenames = clean_lines.iter()
        .filter_map(|line| line.strip_prefix("file '").and_then(|s| s.strip_suffix("'")))
        .map(|s| s.to_string())
        .collect();

    Ok(filenames)
}

/* pub async fn get_file_duration(filename: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let output = Command::new("ffprobe")
    .args(&[
        "-v", "error",
        "-show_entries", "format=duration",
        "-of", "default=noprint_wrappers=1:nokey=1",
        filename
    ])
    .output().await
    .expect("Failed to run ffprobe");

    let duration = String::from_utf8_lossy(&output.stdout).trim().to_string().parse::<f64>()?;
    Ok(duration.round() as u64)
} */

pub async fn join_files(name: &str, output_filename: &str, subtitle_arguments: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut concat_cmd = Command::new("ffmpeg");

    concat_cmd.args(&[
        "-y",
        "-hide_banner",
        "-f", "concat",
        "-safe", "0",
        "-i", format!("{}.join", name).as_str(),
        ]);
    concat_cmd.args(subtitle_arguments);
    concat_cmd.args(&[
        "-c:v", "copy",
        "-c:a", "copy",
        output_filename,
    ]);
    concat_cmd.spawn().expect("Failed to join videos.");

    Ok(())
}

/* pub async fn create_join_file(filenames: Vec<String>, join_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut content = String::new();
    for filename in filenames {
        content.push_str(&format!("file '{}'\n", filename));
    }

    let filename = format!("{}.join", join_file);
    std::fs::write(filename, content)?;
    Ok(())
} */

pub async fn append_to_join_file(filename: &str, join_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .read(true)
        .open(format!("{}.join", join_file))?;

    use std::io::Write;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    if buf.contains(&format!("file '{}'", filename)) {
        return Ok(());
    }

    writeln!(file, "file '{}'", filename)?;
    Ok(())
}

pub async fn create_snapshot_file(filename: &str, elapsed_time_us: u64) -> Result<(), Box<dyn std::error::Error>> {
    let snapshot_filename = format!("{}.snapshot", filename);
    std::fs::write(snapshot_filename, elapsed_time_us.to_string())?;
    Ok(())
}