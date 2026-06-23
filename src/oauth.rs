use std::{collections::HashMap, sync::Arc};
use serde::{Deserialize, Serialize};
use warp::Filter;

use crate::db::DbManager;
use crate::models::*;


#[derive(Debug)]
struct DbError(String);

impl warp::reject::Reject for DbError {}

pub async fn has_auth_token(db: &Arc<DbManager>) -> bool {
    // tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    get_youtube_token(&db).await.is_ok()
}

pub async fn start_oauth_server(db: &Arc<DbManager>) {
    let dbc = db.clone();

    tokio::spawn(async move {
        if let Err(e) = init_youtube_oauth(&dbc).await {
            eprintln!("OAuth init failed: {}", e);
        }

        start_callback_server(dbc).await;
    });
}

async fn start_callback_server(db: Arc<DbManager>) {
    let db_filter = warp::any().map(move || db.clone());

    let callback = warp::path!("auth" / "callback")
        .and(warp::query::<HashMap<String, String>>())
        .and(db_filter)
        .and_then(handle_callback);

    println!("OAuth callback listening on :8080");

    warp::serve(callback)
        .run(([127, 0, 0, 1], 8080))
        .await;
}

async fn handle_callback(
    params: HashMap<String, String>,
    db: Arc<DbManager>,
) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(code) = params.get("code") {
        let mut auth_token = get_youtube_token(&db)
            .await
            .map_err(|e| warp::reject::custom(DbError(e.to_string())))?;

        auth_token.auth_code = Some(code.clone()); // Override auth code

        let _ = exchange_youtube_auth_code(&db, auth_token).await;
    }

    Ok("Authorization successful. You can close this tab.")
}

async fn init_youtube_oauth(
    db: &Arc<DbManager>
) -> Result<(), String> {
    let client_id=  std::env::var("YOUTUBE_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("YOUTUBE_CLIENT_SECRET").unwrap_or_default();

    let auth_token = db.save_oauth_token(&OAuthToken {
        provider: "youtube".to_string(),
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        access_token: None,
        refresh_token: None,
        auth_code: None,
        expires_at: None,
        updated_at: chrono::Utc::now().timestamp(),
    })
    .await
    .map_err(|e| format!("Store YouTube token error: {}", e.to_string()))?;

    match auth_token {
        Some(token) => {
            match &token.auth_code {
                Some(_) => {}
                None => {
                    let url = format!(
                        "https://accounts.google.com/o/oauth2/v2/auth\
                        ?client_id={}\
                        &redirect_uri={}\
                        &response_type=code\
                        &scope=https://www.googleapis.com/auth/youtube.upload\
                        &access_type=offline\
                        &prompt=consent",
                        std::env::var("YOUTUBE_CLIENT_ID").unwrap_or_default(),
                        urlencoding::encode("http://localhost:8080/auth/callback")
                    );

                    println!("Authorize YouTube:");
                    println!("{}", url);

                    let _ = webbrowser::open(&url);
                }
            }
        }
        None => {
            println!("YouTube does not created");
        }
    }

    Ok(())
}

async fn exchange_youtube_auth_code(
    db: &Arc<DbManager>,
    oauth_token: OAuthToken,
) -> Result<OAuthToken, String> {
    let auth_code = oauth_token
        .auth_code
        .as_ref()
        .ok_or_else(|| "Missing auth_code".to_string())?;


    let response = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("client_id", oauth_token.client_id.as_str()),
            ("client_secret", oauth_token.client_secret.as_str()),
            ("code", auth_code.as_str()),
            ("grant_type", "authorization_code"),
            ("redirect_uri", "http://localhost:8080/auth/callback"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();

    let body = response
        .text()
        .await
        .map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!(
            "Exchange auth code failed: {}, body: {}",
            status,
            body
        ));
    }

    let json: serde_json::Value =
        serde_json::from_str(&body)
            .map_err(|e| e.to_string())?;


    let access_token = json["access_token"]
        .as_str()
        .ok_or("Missing access_token")?
        .to_string();


    let refresh_token = json["refresh_token"]
        .as_str()
        .ok_or("Missing refresh_token")?
        .to_string();


    let expires_in = json["expires_in"]
        .as_i64()
        .unwrap_or(3600);


    let now = chrono::Utc::now().timestamp();

    let new_token = OAuthToken {
        provider: oauth_token.provider,
        client_id: oauth_token.client_id,
        client_secret: oauth_token.client_secret,
        access_token: Some(access_token),
        refresh_token: Some(refresh_token),
        auth_code: None,
        expires_at: Some(now + expires_in),
        updated_at: now,
    };


    let saved = db
        .save_oauth_token(&new_token)
        .await
        .map_err(|e| e.to_string())?
        .unwrap();


    Ok(saved)
}

pub async fn get_youtube_token(
    db: &Arc<DbManager>
) -> Result<OAuthToken, String> {
    let token = db
        .get_oauth_token("youtube")
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Youtube token not found")?;

    if let Some(expires) = token.expires_at {
        let now = chrono::Utc::now().timestamp();

        if expires <= now + 300 {
            return refresh_youtube_token(
                db,
                token,
            )
            .await;
        }

    }

    Ok(token)
}

pub async fn refresh_youtube_token(
    db: &Arc<DbManager>,
    oauth_token: OAuthToken,
) -> Result<OAuthToken, String> {
    let refresh_token = oauth_token
        .refresh_token
        .as_ref()
        .ok_or_else(|| "Missing refresh_token".to_string())?;

    let response = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&[
            ("client_id", oauth_token.client_id.as_str()),
            ("client_secret",  oauth_token.client_secret.as_str()),
            ("refresh_token", refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();

    let body = response
        .text()
        .await
        .map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!(
            "Refresh token failed: {}, body: {}",
            status,
            body
        ));
    }
    
    let json: serde_json::Value =
        serde_json::from_str(&body)
            .map_err(|e| e.to_string())?;


    let access_token = json["access_token"]
        .as_str()
        .ok_or("Missing access_token")?
        .to_string();

    let expires_in = json["expires_in"]
        .as_i64()
        .unwrap_or(3600);

    // Google usually doesn't issue new refresh tokens, so keep your old refresh tokens.
    let new_refresh_token = json["refresh_token"]
        .as_str()
        .unwrap_or(refresh_token);

    let now = chrono::Utc::now().timestamp();

    let new_token = OAuthToken {
        provider: oauth_token.provider,
        client_id: oauth_token.client_id,
        client_secret: oauth_token.client_secret,
        access_token: Some(access_token),
        refresh_token: Some(new_refresh_token.to_string()),
        auth_code: oauth_token.auth_code,
        expires_at: Some(now + expires_in),
        updated_at: now,
    };

    let saved = db
        .save_oauth_token(&new_token)
        .await
        .map_err(|e| format!(
            "Store YouTube token error: {}",
            e
        ))?
        .unwrap();


    Ok(saved)
    
}