use reqwest::Client;
use std::path::Path;
use tokio::process::Command;

use crate::enums::*;
use crate::helper::*;
use crate::agents::*;
use crate::models::{Scene, SceneAsset};


pub async fn generate_content(client: &Client, system: &str, user: &str, is_json: bool) -> Result<String, String> {
    gemini::generate_script(client, system, user, is_json).await
}

pub async fn generate_scene(client: &Client, target_path: String, scenes: Vec<Scene>, voice_mode: VoiceMode) -> Result<Vec<SceneAsset>, String> {
    let audio_root = format!("{}/audios", target_path);
    let visual_root = format!("{}/images", target_path);

    tokio::fs::create_dir_all(&visual_root)
        .await
        .map_err(|e| e.to_string())?;

    tokio::fs::create_dir_all(&audio_root)
        .await
        .map_err(|e| e.to_string())?;

    let mut handles = Vec::new();

    for scene in scenes {
        let client = client.clone();
        let target_path = target_path.clone();
        let voice_mode = voice_mode.clone();
        let audio_root = audio_root.clone();
        let visual_root = visual_root.clone();

        handles.push(tokio::spawn(async move {
            // ======================
            // VISUAL GENERATION
            // ======================
            let visual = cloudflare::generate_image(&client, &scene.visual_prompt).await?;

            if detect_media(&visual).is_none() {
                return Err("❌ Invalid visual response".to_string());
            }

            let visual_path = format!(
                "{}/scene_{}.png",
                visual_root,
                scene.scene_id
            );

            tokio::fs::write(&visual_path, visual)
                .await
                .map_err(|e| e.to_string())?;

            // ======================
            // VOICE GENERATION
            // ======================
            let mut audio_paths: Vec<String> = Vec::new();

            match voice_mode {
                VoiceMode::PerSegment => {
                    for (i, segment) in scene.voice_segments.iter().enumerate() {
                        let audio = elevenlabs::generate_tts(
                            &client,
                            &segment.text,
                            Some(&segment.speaker),
                        )
                        .await?;

                        if detect_media(&audio).is_none() {
                            return Err("Invalid audio response".to_string());
                        }

                        let path = format!(
                            "{}/scene_{}_{}.mp3",
                            audio_root,
                            scene.scene_id,
                            i
                        );

                        tokio::fs::write(&path, audio).await
                            .map_err(|e| e.to_string())?;

                        audio_paths.push(path);
                    }
                }

                VoiceMode::SingleVoice => {
                    let text = scene.voice_segments
                        .iter()
                        .map(|v| v.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");

                    let audio = elevenlabs::generate_tts(
                        &client,
                        &text,
                        None,
                    )
                    .await?;

                    if detect_media(&audio).is_none() {
                        return Err("Invalid audio response".to_string());
                    }

                    let path = format!(
                        "{}/scene_{}.mp3",
                        audio_root,
                        scene.scene_id
                    );

                    tokio::fs::write(&path, audio).await
                        .map_err(|e| e.to_string())?;

                    audio_paths.push(path);
                }
            }

            let audio_path = concat_audios(&target_path, scene.scene_id, &audio_paths).await?;

            Ok::<_, String>(SceneAsset {
                scene_id: scene.scene_id,
                audio_path,
                visual_path,
            })
        }));
    }

    let mut results = Vec::new();

    for h in handles {
        results.push(h.await.map_err(|e| e.to_string())??);
    }

    Ok(results)
}

pub async fn render_scene(image: &str, audio: &str, output: &str) -> Result<(), String> {
    println!("🎬 [FFmpeg] Render scene starting");
    println!("   image : {}", image);
    println!("   audio : {}", audio);
    println!("   output: {}", output);

    // 1. Check ffmpeg exists
    let ffmpeg_check = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .await;

    if ffmpeg_check.is_err() {
        return Err("❌ ffmpeg not found in PATH. Please install ffmpeg or set PATH correctly.".to_string());
    }

    // 2. Check input files
    if !Path::new(image).exists() {
        return Err(format!("❌ Missing image file: {}", image));
    }

    if !Path::new(audio).exists() {
        return Err(format!("❌ Missing audio file: {}", audio));
    }

    // 3. Create output directory
    if let Some(parent) = Path::new(output).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("❌ Failed to create output dir: {}", e))?;
    }

    // 4. Run ffmpeg
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-loop", "1",
            "-i", image,
            "-i", audio,
            "-shortest",
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            output,
        ])
        .status()
        .await
        .map_err(|e| format!("❌ Failed to execute ffmpeg: {}", e))?;

    if !status.success() {
        return Err(format!("❌ ffmpeg failed with status: {}", status));
    }

    println!("✅ Scene rendered successfully: {}", output);

    Ok(())
}


pub async fn ffmpeg_render(target_path: String, scene_assets: &Vec<SceneAsset>) -> Result<String, String> {
    let mut video_paths = vec![];
    
    for asset in scene_assets {
        let video_path = format!("{}/videos/scene_{}.mp4", target_path, asset.scene_id);

        // check file exists
        if !tokio::fs::try_exists(&video_path).await.unwrap_or(false) {
            render_scene(
                &asset.visual_path,
                &asset.audio_path,
                &video_path,
            )
            .await?;
        }

        video_paths.push(video_path);
    }

    let list_path = format!("{}/list.txt", &target_path);
    let final_path = format!("{}/final_video.mp4", &target_path);
    
    if !tokio::fs::try_exists(&final_path).await.unwrap_or(false) {
        println!("🎬 FFmpeg rendering: {}", final_path);
        
        tokio::fs::write(
            &list_path,
            video_paths
                .iter()
                .map(|v| {
                    let path = Path::new(v)
                        .strip_prefix(&target_path)
                        .unwrap_or(Path::new(v));

                    format!("file './{}'\n", path.display())
                })
                .collect::<String>()
            )
            .await
            .map_err(|e| e.to_string())?;
        
        let status = tokio::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                &list_path,
                "-c",
                "copy",
                &final_path,
            ])
            .status()
            .await
            .map_err(|e| e.to_string())?;
        
        if !status.success() {
            return Err(format!("FFmpeg failed with status: {}", status));
        }
    }
    
    println!("🎬 FFmpeg rendered successfully");
    Ok(final_path.to_string())
}

async fn concat_audios(
    target_path: &str,
    scene_id: u8,
    audio_paths: &Vec<String>,
) -> Result<String, String> {
    if audio_paths.len() == 1 {
        return Ok(audio_paths[0].clone());
    }

    let list_path = format!(
        "{}/audios/scene_{}_list.txt",
        target_path,
        scene_id
    );

    let merged_path = format!(
        "{}/audios/scene_{}.mp3",
        target_path,
        scene_id
    );
    
    let content = audio_paths
        .iter()
        .map(|p| format!("file '{}'\n", p))
        .collect::<String>();

    tokio::fs::write(&list_path, content)
        .await
        .map_err(|e| e.to_string())?;

    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
            &list_path,
            "-c",
            "copy",
            &merged_path,
        ])
        .status()
        .await
        .map_err(|e| e.to_string())?;

    if !status.success() {
        return Err(format!(
            "Failed to concat audio for scene {}",
            scene_id
        ));
    }

    Ok(merged_path)
}

// pub async fn ffmpeg_render(session_id: &str, images: &[String], audios: &[String]) -> Result<String, String> {
//     let mut scene_videos = vec![];

//     for (i, (img, audio)) in images.iter().zip(audios.iter()).enumerate() {
//         let scene_video = format!("./output/{}/videos/scene_{}.mp4", &session_id, i + 1);

//         render_scene(
//             img,
//             audio,
//             &scene_video
//         ).await?;

//         scene_videos.push(scene_video);
//     }

//     tokio::fs::write(
//         "list.txt",
//         scene_videos
//             .iter()
//             .map(|v| format!("file '{}'\n", v))
//             .collect::<String>()
//     )
//     .await
//     .map_err(|e| e.to_string())?;

//     let final_path = format!("./output/{}/final_video.mp4", &session_id);
//     tokio::process::Command::new("ffmpeg")
//         .args([
//             "-y",
//             "-f",
//             "concat",
//             "-safe",
//             "0",
//             "-i",
//             "list.txt",
//             "-c",
//             "copy",
//             &final_path
//         ])
//         .status()
//         .await
//         .map_err(|e| e.to_string())?;

//     Ok(final_path.to_string())
// }
