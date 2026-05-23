use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn download_site(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let content = response.text().await?;

    Ok(content)
}

pub fn extract_definition_json(content: &str) -> Result<String, Box<dyn std::error::Error>> {
    let re = regex::Regex::new("<script id=\"__NEXT_DATA__\" type=\"application\\/json\">(.*)<\\/script>")?;
    if let Some(caps) = re.captures(content) {
        Ok(caps[1].to_string())
    } else {
        Err("Definition JSON not found".into())
    }
}

pub fn is_movie(content: &str) -> bool {
    let re = regex::Regex::new("\"showType\":\"movie\"").unwrap();
    re.is_match(content)
}

pub fn extract_title(json: &str) -> Result<String, Box<dyn std::error::Error>> {
    let re = regex::Regex::new("\"title\":\"([^\"]*)\"")?;
    if let Some(caps) = re.captures(json) {
        Ok(caps[1].to_string())
    } else {
        Err("Title not found".into())
    }
}

pub fn extract_idec(json: &str) -> Result<String, Box<dyn std::error::Error>> {
    let re = regex::Regex::new("\"idec\":\"([^\"]*)\"")?;
    if let Some(caps) = re.captures(json) {
        Ok(caps[1].to_string())
    } else {
        Err("IDEC not found".into())
    }
}

pub fn extract_year(json: &str) -> Result<String, Box<dyn std::error::Error>> {
    let re = regex::Regex::new("\"year\":\"(\\d+)\"")?;
    if let Some(caps) = re.captures(json) {
        Ok(caps[1].to_string())
    } else {
        Err("Year not found".into())
    }
}

pub fn get_playlist_url(idec: &str) -> String {
    format!("https://api.ceskatelevize.cz/video/v1/playlist-vod/v1/stream-data/media/external/{}?canPlayDrm=true", idec)
}

pub fn extract_stream_url(json: &str) -> Result<String, Box<dyn std::error::Error>> {
    let re = regex::Regex::new("\"forceSubtitles\":\\w+,\"url\":\"([^\"]*)\"")?;
    if let Some(caps) = re.captures(json) {
        Ok(caps[1].to_string())
    } else {
        Err("Stream URL not found".into())
    }
}

pub fn extract_subtitle_urls(json: &str) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let re = regex::Regex::new("\"language\":\"(\\w+)\".*?\"url\":\"([^\"]*)\",\"format\":\"vtt\"")?;
    let urls = re.captures_iter(json)
        .map(|caps| (caps[1].to_string(), caps[2].to_string()))
        .collect();
    Ok(urls)
}

pub fn sanitize_filename(filename: &str) -> String {
    let re = regex::Regex::new("[<>:\"/\\|?*]").unwrap();
    re.replace_all(filename, "_").to_string()
}

pub fn format_title_year(title: &str, year: &str) -> String {
    let sanitized_title = sanitize_filename(title);
    format!("{} ({})", sanitized_title, year)
}

pub fn format_episode(show_title: &str, season_title: &str, episode_title: &str) -> String {
    let sanitized_show_title = sanitize_filename(show_title);
    let sanitized_season_title = sanitize_filename(season_title);
    let sanitized_episode_title = sanitize_filename(episode_title);
    format!("{} - {} - {}", sanitized_show_title, sanitized_season_title, sanitized_episode_title)
}

pub fn get_output_filename(name: &str) -> String {
    let sanitized_name = sanitize_filename(name);
    format!("{}.mp4", sanitized_name)
}

pub fn get_subtitle_filename(name: &str) -> String {
    let sanitized_name = sanitize_filename(name);
    format!("{}.vtt", sanitized_name)
}

pub async fn download_subtitle(url: &str, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let content = response.text().await?;

    std::fs::write(filename, content)?;

    Ok(())
}

pub async fn download_playlist(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let content = response.text().await?;

    Ok(content)
}

pub async fn download_manifest(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let content = response.text().await?;

    Ok(content)
}

pub fn extract_video_qualities(manifest: &str) -> Vec<i32> {
    let re = regex::Regex::new("<Representation id=\"[\\d-]+\" codecs=\"[\\w\\d\\.]+\" width=\"\\d+\" height=\"(\\d+)\" sar=\"[\\d:]+\" bandwidth=\"\\d+\"\\/>").unwrap();
    re.captures_iter(manifest)
        .map(|caps| caps[1].parse::<i32>().unwrap_or(0))
        .collect()
}

pub fn extract_audio_languages(manifest: &str) -> Vec<String> {
    let re = regex::Regex::new("<AdaptationSet .* lang=\"(\\w+)\" >").unwrap();
    re.captures_iter(manifest)
        .map(|caps| caps[1].to_string())
        .collect()
}

pub fn create_mapping_arguments(video_qualities: Vec<i32>, languages: Vec<String>) -> Vec<String> {
    let mut args: Vec<String> = vec![];

    let video_quality_count = video_qualities.len();

    args.push("-map".into());
    args.push(format!("0:{}", video_quality_count - 1));

    for i in 0..languages.len() {
        args.push("-map".into());
        args.push(format!("0:{}", video_quality_count + i));
    }

    args
}

pub fn create_ffmpeg_arguments(stream_url: &str, subtitle_arguments: Vec<String>, mapping_arguments: Vec<String>, output_filename: &str) -> Vec<String> {
    let mut args: Vec<String> = vec![];

    args.push("-headers".into());
    args.push("Referer: https://player.ceskatelevize.cz".into());
    args.push("-user_agent".into());
    args.push("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".into());
    args.push("-y".into());
    args.push("-hide_banner".into());
    args.push("-readrate".into());
    args.push("2".into());
    args.push("-max_interleave_delta".into());
    args.push("100000".into());
    args.push("-avoid_negative_ts".into());
    args.push("make_zero".into());
    args.push("-fflags".into());
    args.push("+genpts+igndts".into());
    args.push("-timeout".into());
    args.push("3".into());
    args.push("-reconnect".into());
    args.push("1".into());
    args.push("-reconnect_at_eof".into());
    args.push("1".into());
    args.push("-reconnect_delay_max".into());
    args.push("2".into());
    args.push("-i".into());
    args.push(format!("{}", stream_url));
    args.extend(subtitle_arguments);
    args.extend(mapping_arguments);
    args.push(format!("{}", output_filename));

    args
}

pub fn create_subtitle_arguments(subtitle_files: &HashMap<String, String>) -> Vec<String> {
    let mut args: Vec<String> = vec![];

    let mut index = 1;

    for (language, filename) in subtitle_files {
        args.push("-i".into());
        args.push(filename.into());
        args.push("-map".into());
        args.push(format!("{}:0", index));
        args.push(format!("-metadata:s:s:{}", index - 1));
        args.push(format!("language={}", language));

        index += 1;
    }
    
    args.push("-c:s".into());
    args.push("mov_text".into());

    args
}

pub async fn run_command(args: Vec<String>, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Arguments: {:?}", args);

    let mut child = Command::new("ffmpeg")
        .args(args)
       /*  .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped()) */
        .spawn()
        .expect("Failed to start command");

    /* let stderr = child.stderr.take().expect("No stderr");
    let mut reader = BufReader::new(stderr).lines();

    let name = name.to_string();
    
    tokio::spawn(async move {
        while let Some(line) = reader.next_line().await.unwrap() {
            if line.contains("time") || line.contains("error") {
                println!("{}: {}", name, line);
            }
        }
    }); */

    /* let status = child.wait().await?;
    if !status.success() {
        return Err(format!("Command failed with status: {}", status).into());
    } */

    Ok(())
}