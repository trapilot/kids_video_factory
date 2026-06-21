use std::env;
use rand::Rng;
use reqwest::{Client, Url};

pub async fn generate_image(
    client: &Client,
    prompt: &str,
) -> Result<Vec<u8>, String> {
    let api_key =
        env::var("POLLINATIONS_API_KEY")
            .map_err(|_| "Missing POLLINATIONS_API_KEY")?;


    let seed = rand::thread_rng().gen::<u32>();

    let mut url = Url::parse("https://image.pollinations.ai/prompt/")
        .map_err(|e| e.to_string())?;

    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| "Cannot modify URL path".to_string())?;

        segments.push(prompt);
    }

    url.query_pairs_mut()
        .append_pair("width", "1024")
        .append_pair("height", "1024")
        .append_pair("seed", &seed.to_string())
        .append_pair("nologo", "true");

    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Pollinations request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Pollinations returned status {}",
            response.status()
        ));
    }

    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("Failed to read image bytes: {}", e))
}