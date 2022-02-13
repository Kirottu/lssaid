use std::{
    env,
    fs::File,
    io::{Read, Write},
    time::Duration,
};

use clap::{app_from_crate, Arg};
use serde_json::Value;
use unicode_segmentation::UnicodeSegmentation;

const NOT_VALID: &str = "\x1B[31mNot a valid Steam AppID*!\x1B[0m";
const CACHE_LOCATION: &str = ".cache/lssaid_cache.json";

// A .expect() function replacement for Results which has a lot nicer output
macro_rules! result_match {
    ( $result:expr, $message:expr ) => {
        match $result {
            Ok(ok) => ok,
            Err(why) => {
                println!("\x1B[1;31m{}:\x1B[0m {}", $message, why);
                std::process::exit(1);
            }
        }
    };
}

fn main() {
    // Get the arguments with clap
    let matches = app_from_crate!()
        .arg(Arg::new("parse-provided-ids")
             .short('i')
             .long("parse-provided-ids")
             .takes_value(true)
             .conflicts_with_all(&["search", "directory"])
             .multiple_values(true)
             .help("Do not parse current or specified directory names, only IDs provided to this argument"))
        .arg(Arg::new("search")
             .short('s')
             .long("search")
             .takes_value(true)
             .conflicts_with_all(&["parse-provided-ids", "directory"])
             .multiple_values(true)
             .help("Search for app names in the Steam app list. Please note this is case sensitive"))
        .arg(Arg::new("refresh")
             .short('r')
             .long("refresh")
             .takes_value(false)
             .help("Refresh the appid cache"))
        .arg(Arg::new("directory")
             .takes_value(true)
             .conflicts_with_all(&["parse-provided-ids", "search"])
             .default_value("./")
             .help("The directory to match file names from, defaults to current directory")).get_matches();

    // Get the app list string.
    // If the refresh parameter is present, do that, if the file has been last modified 2 weeks ago
    // also refresh and if the cache file does not exist at all refresh too
    let app_list_str = if matches.is_present("refresh") {
        fetch_steam_app_list()
    } else if let Ok(mut file) = File::open(format!(
        "{}/{}",
        result_match!(
            env::var("HOME"),
            "No $HOME variable set, unable to determine home directory"
        ),
        CACHE_LOCATION
    )) {
        if result_match!(
            result_match!(
                result_match!(file.metadata(), "Failure reading file metadata").modified(),
                "Failure reading date of modification from the file metadata"
            )
            .elapsed(),
            "Failure getting time elapsed from the modification time"
        ) > Duration::from_secs(1209600)
        // 2 weeks in seconds
        {
            fetch_steam_app_list()
        } else {
            // Read the cache file
            let mut buffer = String::new();
            result_match!(
                file.read_to_string(&mut buffer),
                "Failed to read from cache file"
            );
            buffer
        }
    } else {
        fetch_steam_app_list()
    };

    // Parse the raw json string into a serde Value struct
    let app_list: Value = result_match!(
        serde_json::from_str(&app_list_str),
        "Unable to parse the app list json file"
    );

    // Perform the actual searching through the app list, perform certain actions dependent on the
    // operation selected
    if matches.is_present("parse-provided-ids") {
        // Form the final vector that the found names will be placed into
        let mut provided_appid_list: Vec<(String, String)> = matches
            .values_of("parse-provided-ids")
            .unwrap()
            .map(|val| (val.to_string(), NOT_VALID.to_string()))
            .collect();
        // Loop through the app list to match the names to the specified IDs
        for object in app_list["applist"]["apps"].as_array().unwrap().iter() {
            for appid in &mut provided_appid_list {
                if object["appid"].as_u64().unwrap().to_string() == appid.0 {
                    appid.1 = object["name"].as_str().unwrap().to_string();
                }
            }
        }
        // Finally print them out
        print_list(provided_appid_list);
    } else if matches.is_present("search") {
        let mut match_list: Vec<(String, String)> = Vec::new();
        let names_to_search: Vec<String> = matches
            .values_of("search")
            .unwrap()
            .map(|val| val.to_string())
            .collect();

        for object in app_list["applist"]["apps"].as_array().unwrap().iter() {
            for name in &names_to_search {
                if object["name"].as_str().unwrap().contains(name) {
                    match_list.push((
                        object["name"]
                            .as_str()
                            .unwrap()
                            .replace(name, &format!("\x1B[1m{}\x1B[0m", name)),
                        object["appid"].as_u64().unwrap().to_string(),
                    ));
                }
            }
        }
        print_list(match_list);
    } else {
        let mut files: Vec<(String, String)> = result_match!(
            std::fs::read_dir(matches.value_of("directory").unwrap()),
            "Failed to read directory files"
        )
        .map(|rdir| {
            (
                result_match!(rdir, "Unable to read directory item")
                    .file_name()
                    .to_str()
                    .unwrap()
                    .to_string(),
                NOT_VALID.to_string(),
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

        print_list(files);
    }
}

// Fetches the steam app list and writes it to the cache
fn fetch_steam_app_list() -> String {
    println!("Refreshing app list cache!");
    let client = reqwest::blocking::Client::new();
    let app_list_str = result_match!(
        result_match!(
            client
                .get("https://api.steampowered.com/ISteamApps/GetAppList/v2/")
                .send(),
            "Failed to fetch the steam app list"
        )
        .text(),
        "Failed to get text from the request response"
    );

    let mut file = result_match!(
        File::create(format!("{}/{}", env::var("HOME").unwrap(), CACHE_LOCATION)),
        "Failed to open cache file for writing"
    );

    result_match!(
        file.write_all(app_list_str.as_bytes()),
        "Failed to write to cache file"
    );

    println!(
        "\x1B[1;32mRefresh done!\x1B[0m Saved into \x1B[1m{}/{}\x1B[0m",
        result_match!(
            env::var("HOME"),
            "No $HOME variable found, unable to determine home directory"
        ),
        CACHE_LOCATION
    );

    app_list_str
}

fn print_list(mut list: Vec<(String, String)>) {
    let mut longest_item = 0;
    for item in &list {
        let item_len = std::str::from_utf8(&strip_ansi_escapes::strip(item.0.clone()).unwrap())
            .unwrap()
            .graphemes(true)
            .count();
        if item_len > longest_item {
            longest_item = item_len;
        }
    }

    for item in &mut list {
        let length_difference = longest_item
            - std::str::from_utf8(&strip_ansi_escapes::strip(item.0.clone()).unwrap())
                .unwrap()
                .graphemes(true)
                .count();

        for _ in 0..length_difference {
            item.0.push(' ');
        }
        println!("{} \x1B[1;32m->\x1B[0m {}", item.0, item.1);
    }
}
