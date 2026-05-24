use std::collections::HashMap;

use indicatif::MultiProgress;

use crate::common;

pub async fn download_movie(url: &String, json: &String) -> Result<(), Box<dyn std::error::Error>> {
    let title = common::extract_title(json)?;
    let idec = common::extract_idec(json)?;
    let year = common::extract_year(json)?;
    let total_duration = common::extract_total_duration(json)?;

    println!("Downloading movie '{} ({})'.", title, year);

    let name = common::format_title_year(&title, &year);
    
    download_with_idec(&idec, &name, total_duration, MultiProgress::new(), reqwest::Client::new()).await
}

pub async fn download_with_idec(idec: &String, name: &String, total_duration: u64, m: MultiProgress, client: reqwest::Client) -> Result<(), Box<dyn std::error::Error>> {
    let playlist_url = common::get_playlist_url(&idec);
    let playlist = common::download_playlist(&playlist_url, client.clone()).await?;

    let stream_url = common::extract_stream_url(&playlist)?;

    let mut subtitle_files = HashMap::<String, String>::new();

    if let Ok(subtitle_urls) = common::extract_subtitle_urls(&playlist) {
        println!("[{}] Found {} subtitles.", name, subtitle_files.len());

        for (language, subtitle_url) in subtitle_urls {
            let subtitle_filename = common::get_subtitle_filename(&format!("{} - {}", &name, language));
            println!("[{}] Downloading subtitles to '{}'.", name, subtitle_filename);
            common::download_subtitle(&subtitle_url, &subtitle_filename, client.clone()).await?;

            subtitle_files.insert(language, subtitle_filename);
        }
    } else {
        println!("[{}] No subtitles found.", name);
    }

    let manifest = common::download_manifest(&stream_url, client.clone()).await?;
    let output_filename = common::get_output_filename(&name);

    let video_qualities = common::extract_video_qualities(&manifest);

    let languages = common::extract_audio_languages(&manifest);

    println!("[{}] Available video qualities: {}", name, video_qualities.iter().map(|e| format!("{}p", e)).collect::<Vec<String>>().join(", "));
    println!("[{}] Available audio languages: {}", name, languages.join(", "));

    let mapping_arguments = common::create_mapping_arguments(video_qualities, languages);
    let subtitle_arguments = common::create_subtitle_arguments(&subtitle_files);

    let args = common::create_ffmpeg_arguments(&stream_url, subtitle_arguments, mapping_arguments, &output_filename);

    common::run_command(args, &name, m, total_duration).await;
    Ok(())
}