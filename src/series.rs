use crate::common;
use crate::movie::download_with_idec;

pub async fn download_series(url: &String, json: &String) -> Result<(), Box<dyn std::error::Error>> {
    let data = serde_json::from_str::<serde_json::Value>(json)?;

    let show_count = data["props"]["pageProps"]["data"]["show"]["playableEpisodeCount"].as_u64().unwrap_or(0);
    let title = data["props"]["pageProps"]["data"]["show"]["title"].as_str().unwrap_or("");

    println!("Found series '{}' with {} episodes.", title, show_count);

    let idec = common::extract_idec(json)?;
    let episodes_json = fetch_episodes(&idec, show_count).await?;

    let episodes = episodes_json["data"]["episodesPreviewFind"]["items"].as_array();
    let episodes = match episodes {
        Some(eps) => eps,
        None => {
            println!("No episodes found for series '{}'.", title);
            return Ok(());
        }
    };

    let mut id = 0;

    for episode in episodes {
        id += 1;

        let episode_title = episode["title"].as_str().unwrap_or("");
        let show_title = episode["showTitle"].as_str().unwrap_or("");
        let season_title = episode["season"]["title"].as_str().unwrap_or("");

        println!("{}. {} - {}: {}", id, show_title, season_title, episode_title);
    }

    println!("Type an episode number to download, or 'q' to quit:");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();

    if input.eq_ignore_ascii_case("q") {
        println!("Exiting.");
        return Ok(());
    }

    let episode_number: usize = match input.parse() {
        Ok(num) => num,
        Err(_) => {
            println!("Invalid input. Exiting.");
            return Ok(());
        }
    };

    if episode_number == 0 || episode_number > episodes.len() {
        println!("Episode number out of range. Exiting.");
        return Ok(());
    }

    let episode = &episodes[episode_number - 1];
    let episode_idec = episode["id"].as_str().unwrap_or("");

    let episode_title = episode["title"].as_str().unwrap_or("");
    let show_title = episode["showTitle"].as_str().unwrap_or("");
    let season_title = episode["season"]["title"].as_str().unwrap_or("");

    let name = common::format_episode(show_title, season_title, episode_title);
    
    println!("Downloading episode '{} - {}: {}'.", show_title, season_title, episode_title);

    return download_with_idec(&episode_idec.to_string(), &name).await;
}



async fn fetch_episodes(idec: &str, count: u64) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let res = client.post("https://api.ceskatelevize.cz/graphql/")
        .header("Referer", "https://ceskatelevize.cz")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .json(&serde_json::json!({
            "query": "query GetEpisodes($idec: String!, $seasonId: String, $limit: PaginationAmount!, $offset: Int!, $orderBy: EpisodeOrderByType!, $keyword: String, $onlyPlayable: Boolean) {\n  episodesPreviewFind(\n    idec: $idec\n    seasonId: $seasonId\n    limit: $limit\n    offset: $offset\n    orderBy: $orderBy\n    keyword: $keyword\n    onlyPlayable: $onlyPlayable\n  ) {\n    totalCount\n    items {\n      ...EpisodeRowFragment\n      date {\n        datetime\n        channelName\n        prefixText\n        __typename\n      }\n      __typename\n    }\n    __typename\n  }\n}\n\nfragment EpisodeRowFragment on EpisodePreview {\n  description\n  season {\n    title\n    __typename\n  }\n  groups {\n    title\n    __typename\n  }\n  ...VideoCardFragment\n  __typename\n}\n\nfragment VideoCardFragment on EpisodePreview {\n  id\n  cardLabels {\n    topLeft\n    center\n    bottomRight\n    __typename\n  }\n  images {\n    card(width: 480, height: 270)\n    __typename\n  }\n  playable\n  showCode\n  showId\n  showTitle\n  title\n  duration\n  labels {\n    icon\n    text\n    textLong: text(short: false)\n    __typename\n  }\n  __typename\n}",
            "variables": {
                "idec": idec,
                "limit": count,
                "offset": 0,
                "onlyPlayable": true,
                "orderBy": "oldest"
            },
            "extensions": {"persistedQuery":{"version":1,"sha256Hash":"e627db8ae17ccfbb925f298f9d6ba46d80f65fcea7c7d824a87457400a6c3035"}}
        })).send().await?;

    let json: serde_json::Value = res.json().await?;
    Ok(json)
}