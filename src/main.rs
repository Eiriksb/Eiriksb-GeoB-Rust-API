#[macro_use] extern crate rocket;

use rocket::http::Header;
use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::serde::json::Json;
use std::path::PathBuf;
use serde_json::Value;
use geojson::GeoJson;
use std::fs;
use rocket::Config;
use urlencoding::decode;
use std::borrow::Cow;
use strsim::jaro_winkler;
use std::io::Write;
use std::fs::File;
use std::path::Path;
use serde_json::json;

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}



#[get("/update-data")]
async fn update_data() -> Result<String, String> {
    // Define the path to the data/geojsons directory
    let geojsons_dir = "data/geojsons";

    // Check if the data/geojsons directory exists and delete it if it does
    if fs::metadata(geojsons_dir).is_ok() {
        fs::remove_dir_all(geojsons_dir).map_err(|e| format!("Failed to delete existing geojsons directory: {}", e))?;
    }

    // Recreate the data/geojsons directory
    fs::create_dir_all(geojsons_dir).map_err(|e| format!("Failed to create geojsons directory: {}", e))?;

    let adm_levels = vec!["ADM0", "ADM1", "ADM2"];
    let base_url = "https://github.com/wmgeolab/geoBoundaries/raw/main/releaseData/CGAZ/geoBoundariesCGAZ_";

    for level in adm_levels {
        let url = format!("{}{}.geojson", base_url, level);
        let response = reqwest::get(&url).await;

        match response {
            Ok(mut resp) => {
                let total_size = resp.content_length().unwrap_or(0);
                let mut downloaded = 0;

                // Ensure the target directory exists
                let dir_path = format!("{}/{}", geojsons_dir, level);
                fs::create_dir_all(&dir_path).map_err(|e| format!("Failed to create directory: {}", e))?;

                let file_path = format!("{}/{}.geojson", dir_path, level);
                let mut file = fs::File::create(&file_path).map_err(|e| format!("Failed to create file: {}", e))?;

                while let Some(chunk) = resp.chunk().await.map_err(|e| format!("Failed to read chunk: {}", e))? {
                    file.write_all(&chunk).map_err(|e| format!("Failed to write chunk: {}", e))?;
                    downloaded += chunk.len() as u64;

                    // Calculate the percentage of the download completed
                    let percentage = (downloaded as f64 / total_size as f64) * 100.0;

                    // Print the loading bar on the same line
                    print!("\rDownloading {}... {:.2}%", level, percentage);
                    std::io::stdout().flush().unwrap(); // Ensure the output is immediately visible
                    
                }

                println!("\nSuccessfully updated {} data.", level);
            },
            Err(e) => return Err(format!("Failed to download data: {}", e)),
        }
    }

    // Call the function to extract shapes
    match extract_data().await {
        Ok(_) => println!("Data extraction completed successfully."),
        Err(e) => return Err(format!("Failed to extract data: {}", e)),
    }

    // Delete the ADM0.geojson, ADM1.geojson, and ADM2.geojson files
    for level in &["ADM0", "ADM1", "ADM2"] {
        let file_path = format!("{}/{}.geojson", geojsons_dir, level);
        if fs::metadata(&file_path).is_ok() {
            fs::remove_file(&file_path).map_err(|e| format!("Failed to delete {}.geojson: {}", level, e))?;
        }
    }

    Ok("Data update completed.".to_string())
}

#[get("/check-geojsons")]
async fn check_geojsons() -> Json<Value> {
    let geojsons_dir = "data/geojsons";
    let exists = fs::metadata(geojsons_dir).is_ok();
    Json(json!({ "exists": exists }))
}

#[get("/autocomplete?<query>")]
async fn autocomplete(query: String) -> Json<Vec<String>> {
    println!("Autocomplete query: {}", query); // Log the input query

    // Make the query uppercase 
    let query = query.to_uppercase();
    
    let adm1_dir = "data/geojsons/ADM0";
    let entries = match fs::read_dir(adm1_dir) {
        Ok(entries) => entries,
        Err(e) => {
            println!("Error reading directory: {}", e);
            return Json(vec![]);
        },
    };

    let filenames: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().ok().map_or(false, |ft| ft.is_file()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();

    let mut matches_with_scores: Vec<(String, f64)> = Vec::new();

    // Prioritize exact matches
    for filename in &filenames {
        if filename.to_uppercase().contains(&query) {
            matches_with_scores.push((filename.clone(), 1.0)); // Exact match, high score
        }
    }

    // Apply Jaro-Winkler similarity for non-exact matches
    for filename in &filenames {
        if !matches_with_scores.iter().any(|(f, _)| f == filename) {
            let score = jaro_winkler(&query, filename);
            println!("Score for {}: {}", filename, score); // Log the similarity score
            if score > 0.5 {
                matches_with_scores.push((filename.clone(), score));
            }
        }
    }

    // Sort the matches by score in descending order and take the first 5
    matches_with_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let top_5_matches: Vec<String> = matches_with_scores.into_iter().take(5).map(|(filename, _)| filename).collect();

    println!("Top 5 matches: {:?}", top_5_matches); // Log the top 5 matches

    Json(top_5_matches)
}

#[get("/geojson?<iso3>&<query>")]
async fn get_geojson(iso3: String, query: Option<String>) -> Json<Value> {
    // Decode the iso3
    let decoded_iso3 = decode(&iso3).unwrap_or(Cow::Borrowed(&iso3)).into_owned();

    // Call the autocomplete function with the decoded iso3 as the query
    let autocomplete_results = autocomplete(decoded_iso3.clone()).await;

    // Use the first entry from the autocomplete results as the ISO3 code
    let iso3 = if let Some(first_entry) = autocomplete_results.into_inner().get(0) {
        first_entry.clone()
    } else {
        // If no autocomplete results, return an error or a default value
        return Json(json!({"error": "No autocomplete results found for the provided iso3"}));
    };

    // Check if a query is provided
    if let Some(query) = query {
        // Decode the query
        let decoded_query = decode(&query).unwrap_or(Cow::Borrowed(&query)).into_owned();

        // Remoive .geojson from iso3 if extension if present
        let iso3 = iso3.replace(".geojson", "");
        let iso3 = iso3.split(" - ").next().unwrap_or("");

        println!("Decoded iso3: {:?}", iso3);

        // Load all available queries for the given iso3 
        let available_queries = load_available_queries(&iso3);

        println!("Decoded query: {:?}", decoded_query);
        println!("Available queries: {:?}", available_queries);

        // Find the best match based on the input query
        let best_match = find_best_match(&decoded_query, &available_queries);

        println!("Best match: {:?}", best_match);

        // If a best match is found, attempt to return its data
        if let Some(best_match) = best_match {
            return attempt_to_return_data(&iso3, &best_match);
        }
    }

    // If no query is provided or no best match is found, attempt to return the data for the entire country
    attempt_to_return_data(&iso3, "")
}

async fn extract_data() -> Result<String, String> {
    // Call the function to extract shapes from ADM0
    extract_shapes_from_adm0().await?;

    // Call the function to extract shapes from ADM1
    extract_shapes("ADM1", "data/geojsons/ADM1/ADM1.geojson").await?;

    // Call the function to extract shapes from ADM2
    extract_shapes("ADM2", "data/geojsons/ADM2/ADM2.geojson").await?;

    Ok("Data update completed.".to_string())
}


async fn extract_shapes_from_adm0() -> Result<(), String> {
    // Path to the ADM0 GeoJSON file
    let file_path = "data/geojsons/ADM0/ADM0.geojson";

    // Read the file
    let geojson_data = fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Parse the GeoJSON data
    let geojson: GeoJson = serde_json::from_str(&geojson_data).map_err(|e| format!("Failed to parse GeoJSON: {}", e))?;

    // Ensure the target directory exists
    let dir_path = "data/geojsons/ADM0";
    fs::create_dir_all(dir_path).map_err(|e| format!("Failed to create directory: {}", e))?;

    // Initialize progress tracking variables
    let mut features_processed = 0;

    // Iterate over each feature
    if let GeoJson::FeatureCollection(feature_collection) = geojson {
        let total_features = feature_collection.features.len();

        for feature in feature_collection.features {
            // Access the properties of the feature
            let properties = feature.properties.clone().unwrap_or_else(|| serde_json::Map::new());

            // Extract the shapeName
            let shape_name = properties.get("shapeName").and_then(Value::as_str).unwrap_or("unknown");

            // Extract the shapeGroup
            let shape_group = properties.get("shapeGroup").and_then(Value::as_str).unwrap_or("unknown");

            // Put shapeGroup and shapeName together to create a unique name
            let shape_name = format!("{} - {}", shape_group, shape_name);

            // Create a new file for the shape
            let shape_file_path = Path::new(dir_path).join(format!("{}.geojson", shape_name));
            let mut shape_file = File::create(shape_file_path).map_err(|e| format!("Failed to create shape file: {}", e))?;

            // Write the feature to the file
            let feature_json = serde_json::to_string(&feature).map_err(|e| format!("Failed to serialize feature: {}", e))?;
            shape_file.write_all(feature_json.as_bytes()).map_err(|e| format!("Failed to write shape file: {}", e))?;

            // Update progress tracking
            features_processed += 1;
            let percentage = (features_processed as f64 / total_features as f64) * 100.0;
            print!("\rExtracting shapes from ADM0: {:.2}%", percentage);
            std::io::stdout().flush().unwrap();
        }
    }

    println!("Successfully extracted shapes from ADM0."); // Debugging output
    Ok(())
}

async fn extract_shapes(adm_level: &str, geojson_file_path: &str) -> Result<(), String> {
    // Read the GeoJSON file
    let geojson_data = fs::read_to_string(geojson_file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Parse the GeoJSON data
    let geojson: GeoJson = serde_json::from_str(&geojson_data).map_err(|e| format!("Failed to parse GeoJSON: {}", e))?;

    // Ensure the target directory exists
    let base_dir_path = format!("data/geojsons/{}", adm_level);
    fs::create_dir_all(&base_dir_path).map_err(|e| format!("Failed to create directory: {}", e))?;

    // Initialize progress tracking variables
    let mut features_processed = 0;

    // Iterate over each feature
    if let GeoJson::FeatureCollection(feature_collection) = geojson {
        let total_features = feature_collection.features.len();

        for feature in feature_collection.features {
            // Access the properties of the feature
            let properties = feature.properties.clone().unwrap_or_else(|| serde_json::Map::new());

            // Extract the shapeName and shapeGroup
            let shape_name = properties.get("shapeName").and_then(Value::as_str).unwrap_or("unknown");
            let shape_group = properties.get("shapeGroup").and_then(Value::as_str).unwrap_or("unknown");

            // Sanitize the shapeName by replacing special characters with underscores
            let sanitized_shape_name = shape_name.replace("/", "_");

            // Create a new directory for the shapeGroup if it doesn't exist
            let shape_group_dir_path = format!("{}/{}", base_dir_path, shape_group);
            fs::create_dir_all(&shape_group_dir_path).map_err(|e| format!("Failed to create shape group directory: {}", e))?;

            // Create a new file for the shape within the shapeGroup directory
            let shape_file_path = Path::new(&shape_group_dir_path).join(format!("{}.geojson", sanitized_shape_name));
            let mut shape_file = File::create(shape_file_path).map_err(|e| format!("Failed to create shape file: {}", e))?;

            // Write the feature to the file
            let feature_json = serde_json::to_string(&feature).map_err(|e| format!("Failed to serialize feature: {}", e))?;
            shape_file.write_all(feature_json.as_bytes()).map_err(|e| format!("Failed to write shape file: {}", e))?;

            // Update progress tracking
            features_processed += 1;
            let percentage = (features_processed as f64 / total_features as f64) * 100.0;
            print!("\rExtracting shapes from {}: {:.2}%", adm_level, percentage);
            std::io::stdout().flush().unwrap();
        }
        println!("Successfully extracted shapes from {}.", adm_level); // Debugging output
    }

    Ok(())
}

fn attempt_to_return_data(iso3: &str, best_match: &str) -> Json<Value> {
    // Convert the ISO3 code to uppercase to handle case sensitivity
    let iso3_upper = iso3.to_uppercase();

    // Define the possible ADM levels to check
    let adm_levels = vec!["ADM0", "ADM1", "ADM2", "ADM3"];

    // Iterate through each ADM level and attempt to find the data
    for adm_level in adm_levels {
        // Construct the file path for the current ADM level
        let project_root = std::env::current_dir().expect("Failed to get current directory");
        let file_path = project_root.join("data/geojsons")
            .join(adm_level)
            .join(&iso3_upper)
            .join(best_match)
            .with_extension("geojson");

        print!("Attempting to read file: {:?}", file_path);
        // Attempt to read the file for the current ADM level
        match fs::read_to_string(&file_path) {
            Ok(contents) => {
                match serde_json::from_str::<Value>(&contents) {
                    Ok(json_value) => return Json(json_value), // Return the JSON value if found
                    Err(_) => continue, // Continue to the next ADM level if JSON is invalid
                }
            },
            Err(_) => continue, // Continue to the next ADM level if the file is not found
        }
    }

    // Return a default response if no data is found for any ADM level with the file location specified
    Json(Value::String("Data not found at location.".to_string()) )
}

// Placeholder function to load all available queries
fn load_available_queries(iso3: &str) -> Vec<String> {
    let adm_levels = vec!["ADM0", "ADM1", "ADM2", "ADM3"];
    let mut queries = Vec::new();

    for adm_level in adm_levels {
        // Construct the path to the directory containing the queries
        let mut path = PathBuf::from("./data/geojsons");
        path.push(adm_level);
        path.push(iso3);

        // Attempt to read the directory
        if let Ok(entries) = fs::read_dir(&path) {
            // Use filter_map to handle both Ok and Err cases
            let mut adm_queries: Vec<String> = entries.filter_map(Result::ok)
                .filter_map(|entry| {
                    // Check if the entry is a file
                    if entry.file_type().ok()?.is_file() {
                        // Convert the file name to a string and return it
                        Some(entry.file_name().to_string_lossy().into_owned())
                    } else {
                        None
                    }
                })
                .collect();
            queries.append(&mut adm_queries);
        }
    }

    queries
}  

// Placeholder function to find the best match
fn find_best_match(query: &str, available_queries: &[String]) -> Option<String> {
    available_queries.iter()
        .map(|available_query| (available_query, jaro_winkler(query, available_query)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(query, _)| query.clone())
}



#[launch]
fn rocket() -> _ {
    let config = Config::release_default();

    // Retrieve the PORT environment variable and parse it to an integer
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8081);

    rocket::custom(config)
        .mount("/", routes![get_geojson, update_data, check_geojsons, autocomplete])
        .attach(CORS)
        .configure(rocket::Config {
            address: "0.0.0.0".parse().unwrap(),
            port,
            ..rocket::Config::default()
        })  
}