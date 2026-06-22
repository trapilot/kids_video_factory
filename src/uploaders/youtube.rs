use crate::uploader::*;

pub struct YoutubeUploader {
    client_id: String,
    client_secret: String,
    refresh_token: String,
}

impl YoutubeUploader {
    pub fn new() -> Self {
        Self {
            client_id: std::env::var("YOUTUBE_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("YOUTUBE_CLIENT_SECRET").unwrap_or_default(),
            refresh_token: std::env::var("YOUTUBE_REFRESH_TOKEN").unwrap_or_default(),
        }
    }
}

#[async_trait::async_trait]
impl Uploader for YoutubeUploader {
    fn channel(&self) -> Channel {
        Channel::Youtube
    }
    
    async fn upload(
        &self,
        video_path: &str,
        title: &str,
        config: ChannelConfig
    ) -> Result<(), String> {
        println!("Uploading Youtube");
        
        let metadata = match &config {
            ChannelConfig::Youtube {
                category_id,
                description,
                tags,
            } => {
                serde_json::json!({
                    "snippet": {
                        "title": &title,
                        "description": description,
                        "tags": tags,
                        "categoryId": category_id,
                    },
                    "status": {
                        "privacyStatus": "public",
                        "madeForKids": true,
                        "selfDeclaredMadeForKids": true
                    }
                })
            }
            _ => {
                return Err("Youtube config incorrect".to_string());
            }
        };

        let token_res = reqwest::Client::new()
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", self.client_id.to_string()),
                ("client_secret", self.client_secret.to_string()),
                ("refresh_token", self.refresh_token.to_string()),
                ("grant_type", "refresh_token".to_string()),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let token_json: serde_json::Value = token_res.json().await.map_err(|e| e.to_string())?;
        let access_token = token_json["access_token"]
            .as_str()
            .ok_or("missing access token")?;
        
        let video_bytes = tokio::fs::read(video_path)
            .await
            .map_err(|e| e.to_string())?;

        let boundary = "foo_bar_baz";

        let mut full_body = Vec::new();
        full_body.extend(format!(
            "--{b}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n",
            b = boundary
        ).as_bytes());
        full_body.extend(serde_json::to_string(&metadata).unwrap().as_bytes());
        full_body.extend(b"\r\n");
        full_body.extend(format!(
            "--{b}\r\nContent-Type: video/mp4\r\n\r\n",
            b = boundary
        ).as_bytes());
        full_body.extend(&video_bytes);
        full_body.extend(format!("\r\n--{b}--\r\n", b = boundary).as_bytes());

        let res = reqwest::Client::new()
            .post("https://www.googleapis.com/upload/youtube/v3/videos?uploadType=multipart&part=snippet,status")
            .bearer_auth(access_token)
            .header("Content-Type", format!("multipart/related; boundary={}", boundary))
            .body(full_body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if res.status().is_success() {
            println!("Youtube Upload OK!");
            Ok(())
        } else {
            Err(res.status().to_string())
        }
    }
}