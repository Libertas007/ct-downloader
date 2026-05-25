use std::{collections::HashMap, fs, io::Read};

use indicatif::MultiProgress;
use tokio::{process::Command};

use anyhow::Result;

use crate::movie::download_with_idec;

pub async fn download_loop(idec: &String, name: &String, total_duration: u64, m: MultiProgress, client: reqwest::Client) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match resume_download(idec, name, total_duration, m.clone(), client.clone()).await {
            Ok(_) => break,
            Err(e) => {
                println!("Error during download: {}. Retrying...", e);
            }
        }
    }

    let output_filename = crate::common::get_output_filename(name.as_str());

    join_downloaded_files(name, &output_filename).await?;

    Ok(())
}

pub async fn resume_download(idec: &String, name: &String, total_duration: u64, m: MultiProgress, client: reqwest::Client) -> Result<(), Box<dyn std::error::Error>> {
    let join_file = format!("{}.join", name);

    if let Ok(v) = fs::exists(&join_file) && v {   
        let filenames = read_join_file(&join_file).await?;
        let durations = get_durations(filenames).await?;
        
        let max_duration = *durations.values().max().unwrap_or(&0);
        
        download_with_idec(idec, name, total_duration, m, client, max_duration).await
    } else {
        download_with_idec(idec, name, total_duration, m, client, 0).await
    }
}

pub async fn join_downloaded_files(name: &String, output_filename: &String) -> Result<(), Box<dyn std::error::Error>> {
    let join_file = format!("{}.join", name);
    join_files(&name, output_filename).await?;
    
    let filenames = read_join_file(&join_file).await?;

    for filename in filenames {
        std::fs::remove_file(&filename)?;
        std::fs::remove_file(format!("{}.snapshot", &filename))?;
    }

    std::fs::remove_file(join_file)?;
    Ok(())
}

pub async fn get_durations(filenames: Vec<String>) -> Result<HashMap<String, u64>, Box<dyn std::error::Error>> {
    let mut duration_map = HashMap::new();

    for filename in filenames {
        let duration = get_file_duration(&filename).await?;
        duration_map.insert(filename, duration);
    }

    Ok(duration_map)
}

pub async fn read_join_file(join_file: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(format!("{}.join", join_file))?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let filenames = content.lines()
        .filter_map(|line| line.strip_prefix("file '").and_then(|s| s.strip_suffix("'")))
        .map(|s| s.to_string())
        .collect();

    Ok(filenames)
}

pub async fn get_file_duration(filename: &str) -> Result<u64, Box<dyn std::error::Error>> {
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
}

pub async fn join_files(join_file: &str, output_filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut concat_cmd = Command::new("ffmpeg");

    concat_cmd.args(&[
        "-y",
        "-hide_banner",
        "-f", "concat",
        "-safe", "0",
        "-i", format!("{}.join", join_file).as_str(),
        "-c", "copy",
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