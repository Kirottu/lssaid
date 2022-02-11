use std::{
    env,
    fs::File,
    io::{Read, Write},
    time::Duration,
};

use clap::{app_from_crate, Arg};
use serde_json::Value;

#[tokio::main]
async fn main() {
    // Get the arguments with clap
    let matches = app_from_crate!()
        .arg(Arg::new("parse-provided-ids")
             .short('i')
             .long("parse-provided-ids")
             .takes_value(true)
             .multiple_values(true)
             .help("Do not parse current or specified directory names, only IDs provided to this argument"))
        .arg(Arg::new("refresh")
             .short('r')
             .long("refresh")
             .takes_value(false)
             .help("Refresh the appid cache"))
        .arg(Arg::new("directory")
             .takes_value(true)
             .default_value("./")
             .help("The directory to match file names from, defaults to current directory")).get_matches();

    // Get the app list string.
    // If the refresh parameter is present, do that, if the file has been last modified 2 weeks ago
    // also refresh and if the cache file does not exist at all refresh too
    let app_list_str = if matches.is_present("refresh") {
        fetch_steam_app_list().await
    } else if let Ok(mut file) = File::open(format!(
        "{}/.cache/lssaid-cache.json",
        env::var("HOME").expect("No $HOME variable set, unable to determine home directory")
    )) {
        if file
            .metadata()
            .expect("Failure reading file metadata")
            .modified()
            .expect("Failure reading date of modification from the file metadata")
            .elapsed()
            .expect("Failure getting time elapsed from the date of modification")
            > Duration::from_secs(1209600)
        // 2 weeks in seconds
        {
            fetch_steam_app_list().await
        } else {
            // Read the cache file
            let mut buffer = String::new();
            file.read_to_string(&mut buffer)
                .expect("Unable to read steam app list from the cache file");
            buffer
        }
    } else {
        fetch_steam_app_list().await
    };

    let app_list: Value = serde_json::from_str(&app_list_str).unwrap();
    if !matches.is_present("parse-provided-ids") {
        let mut files: Vec<(String, String)> =
            std::fs::read_dir(matches.value_of("directory").unwrap())
                .unwrap()
                .map(|rdir| {
                    (
                        rdir.unwrap().file_name().to_str().unwrap().to_string(),
                        "Not a valid Steam appid!".to_string(),
                    )
                })
                .collect();
        for object in app_list["applist"]["apps"].as_array().unwrap().iter() {
            for file in &mut files {
                if object["appid"].as_u64().unwrap().to_string() == file.0 {
                    file.1 = object["name"].as_str().unwrap().to_string();
                }
            }
        }

        for file in &files {
            println!("{} -> {}", file.0, file.1);
        }
    } else {
        let mut provided_appid_list: Vec<(String, String)> = matches
            .values_of("parse-provided-ids")
            .unwrap()
            .map(|val| (val.to_string(), "Not a valid Steam appid!".to_string()))
            .collect();
        for object in app_list["applist"]["apps"].as_array().unwrap().iter() {
            for appid in &mut provided_appid_list {
                if object["appid"].as_u64().unwrap().to_string() == appid.0 {
                    appid.1 = object["name"].as_str().unwrap().to_string();
                }
            }
        }
        for appid in &provided_appid_list {
            println!("{} -> {}", appid.0, appid.1);
        }
    }
}

// Fetches the steam app list and writes it to the cache
async fn fetch_steam_app_list() -> String {
    println!("Refreshing app list cache!");
    let client = reqwest::Client::builder().build().unwrap();
    let app_list_str = client
        .get("https://api.steampowered.com/ISteamApps/GetAppList/v2/")
        .send()
        .await
        .expect("Failed to fetch the steam app list")
        .text()
        .await
        .expect("Failed to get text from the request response");

    let mut file = File::create(format!(
        "{}/.cache/lssaid-cache.json",
        env::var("HOME").unwrap()
    ))
    .expect("Failed to open cache file for writing");

    file.write_all(app_list_str.as_bytes()).unwrap();

    println!(
        "Refresh done! Saved into {}/.cache/lssaid-cache.json",
        env::var("HOME").expect("No $HOME variable found, unable to determine home directory")
    );

    app_list_str
}
