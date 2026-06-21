use reqwest::{Client, header};
use serde_json::{json, Value};
use std::env;

use crate::{entities::*, workflow};
use crate::provider;

pub async fn upload_to_youtube(ctx: &workflow::Context, video_path: &str, payload: YoutubePayload) -> Result<(), String> {
    let client_id = env::var("YOUTUBE_CLIENT_ID").map_err(|e| e.to_string())?;
    let client_secret = env::var("YOUTUBE_CLIENT_SECRET").map_err(|e| e.to_string())?;
    let refresh_token = env::var("YOUTUBE_REFRESH_TOKEN").map_err(|e| e.to_string())?;


    if let Some(guard) = ctx.pm.acquire(provider::Provider::Youtube).await {
        let provider = guard.provider.clone();
        let api_key = guard.credential.api_key.clone();

        let result = guard.call(req)::generate_script(
            &ctx.http,
            &api_key,
            system,
            user,
            is_json,
        )
        .await;
    }

    let client = ctx.http
        .post("https://oauth2.googleapis.com/token")
        // .form(&[
        //     ("client_id", client_id.as_str()),
        //     ("client_secret", client_secret.as_str()),
        //     ("refresh_token", refresh_token.as_str()),
        //     ("grant_type", "refresh_token"),
        // ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 1. Get access token
    let token_res = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let token_json: serde_json::Value = token_res.json().await.map_err(|e| e.to_string())?;
    let access_token = token_json["access_token"]
        .as_str()
        .ok_or("missing access token")?;
    
    // 2. Read video
    let video_bytes = tokio::fs::read(video_path)
        .await
        .map_err(|e| e.to_string())?;

    // 3. Metadata
    let metadata = json!({
        "snippet": {
            "title": &payload.title,
            "description": &payload.description,
            "tags": &payload.tags,
            "categoryId": &payload.category_id,
        },
        "status": {
            "privacyStatus": "public",
            "madeForKids": true,
            "selfDeclaredMadeForKids": true
        }
    });

    // 4. Build RAW multipart/related body manually
    let boundary = "foo_bar_baz";
    let body = format!(
        "--{b}\r\n\
Content-Type: application/json; charset=UTF-8\r\n\r\n\
{}\r\n\
--{b}\r\n\
Content-Type: video/mp4\r\n\r\n",
        metadata.to_string(),
        b = boundary
    );

    let mut full_body = body.into_bytes();
    full_body.extend(video_bytes);
    full_body.extend(format!("\r\n--{boundary}--\r\n").into_bytes());

    // 5. Upload
    let res = client
        .post("https://www.googleapis.com/upload/youtube/v3/videos?uploadType=multipart&part=snippet,status")
        .bearer_auth(access_token)
        .header("Content-Type", format!("multipart/related; boundary={}", boundary))
        .body(full_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(res.status().to_string())
    }
}

pub async fn upload_to_tiktok(ctx: &workflow::Context, video_path: &str, payload: TiktokPayload) -> Result<(), String> {
    let access_token = env::var("TIKTOK_ACCESS_TOKEN").unwrap();
    let video_bytes = tokio::fs::read(video_path).await.map_err(|e| e.to_string())?;
    let size = video_bytes.len();

    let init_payload = json!({
        "post_info": {
            "title": &payload.title,
            "privacy_level": &payload.privacy_level,
            "disable_comment": &payload.disable_comment,
        },
        "source_info": {
            "source": "FILE_UPLOAD",
            "video_size": size,
            "chunk_size": size,
            "total_chunk_count": 1,
        }
    });

    let init_res = client.post("https://open.tiktokapis.com/v2/post/publish/video/init/")
        .bearer_auth(&access_token)
        .header(header::CONTENT_TYPE, "application/json; charset=UTF-8")
        .json(&init_payload).send().await.map_err(|e| e.to_string())?;

    let init_json: Value = init_res.json().await.unwrap();
    if init_json["error"]["code"].as_str().unwrap_or("") != "ok" { return Err("TikTok Init Failed".to_string()); }

    let upload_url = init_json["data"]["upload_url"].as_str().unwrap();
    let put_res = client.put(upload_url)
        .header(header::CONTENT_TYPE, "video/mp4")
        .header(header::CONTENT_LENGTH, size.to_string())
        .header("Content-Range", format!("bytes 0-{}/{}", size - 1, size))
        .body(video_bytes).send().await.map_err(|e| e.to_string())?;

    if put_res.status().is_success() {
        println!("🟢 TikTok Upload OK!"); Ok(())
    } else {
        Err(put_res.status().to_string())
    }
}