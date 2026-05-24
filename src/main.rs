use std::env;

mod common;
mod movie;
mod series;

fn main() {
    if !test_ffmpeg() {
        eprintln!("ffmpeg is not installed or not found in PATH. Please install ffmpeg to use this program.");
        std::process::exit(1);
    }

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <URL>", args[0]);
        std::process::exit(1);
    }
    let url = &args[1];
    println!("Downloading from {}.", url);

    match tokio::runtime::Runtime::new().unwrap().block_on(run(url)) {
        Ok(_) => println!("Program finished!"),
        Err(e) => eprintln!("Error: {}", e),
    }
}

async fn run(url: &String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Downloading webpage {}.", url);
    let content = common::download_site(url).await?;
    
    let definition_json = common::extract_definition_json(&content)?;

    if common::is_movie(&definition_json) {
        movie::download_movie(url, &definition_json).await?;
    } else {
        series::download_series(url, &definition_json).await?;
    }

    Ok(())
}

fn test_ffmpeg() -> bool {
    match std::process::Command::new("ffmpeg").arg("-version").output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}