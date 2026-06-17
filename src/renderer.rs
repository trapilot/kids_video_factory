use reqwest::Client;
use std::path::Path;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Semaphore;

use crate::enums::*;
use crate::helper::*;
use crate::apis::*;
use crate::models::{Scene, VideoTimeline, VoiceSegment};

pub async fn build_content(client: &Client, system: &str, user: &str, is_json: bool) -> Result<String, String> {
    gemini::generate_script(client, system, user, is_json).await
}

pub async fn build_timelines(client: &Client, target_path: String, scenes: Vec<Scene>, voice_mode: VoiceMode) -> Result<Vec<VideoTimeline>, String> {
    let audio_root = format!("{}/audios", target_path);
    let visual_root = format!("{}/images", target_path);

    tokio::fs::create_dir_all(&visual_root)
        .await
        .map_err(|e| e.to_string())?;

    tokio::fs::create_dir_all(&audio_root)
        .await
        .map_err(|e| e.to_string())?;

    let mut handles = Vec::new();
    let tts_semaphore = Arc::new(Semaphore::new(2)); // ElevenLabs free max 2 concurrent

    for scene in scenes {
        let client = client.clone();
        let voice_mode = voice_mode.clone();
        let audio_root = audio_root.clone();
        let visual_root = visual_root.clone();

        let semaphore = tts_semaphore.clone();

        handles.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();

            let visual_path = format!("{}/scene_{}.png", visual_root, scene.scene_id);
            let audio_path = format!("{}/scene_{}.mp3", audio_root, scene.scene_id);

            generate_scene_visual(
                &client,
                &visual_path,
                &scene.visual_prompt,
            ).await?;

            generate_scene_audio(
                &client,
                &audio_path,
                &voice_mode,
                &scene.voice_segments,
            ).await?;

            let duration = get_audio_duration(&audio_path).await.map_err(|e| e.to_string())?;

            Ok::<_, String>(VideoTimeline {
                scene_id: scene.scene_id,
                transition: scene.transition,
                motion: scene.motion,
                audio_path,
                visual_path,
                duration,
                start_time: 0.0,
                end_time: 0.0,
            })
        }));
    }

    let mut assets = Vec::new();

    for handle in handles {
        assets.push(
            handle
                .await
                .map_err(|e| e.to_string())??
        );
    }

    assets.sort_by_key(|v| v.scene_id);

    let mut cursor = 0.0;
    let mut timelines = Vec::new();
  
    for asset in assets {
        let start_time = cursor;
        let end_time = start_time + asset.duration;

        timelines.push(VideoTimeline {
            scene_id: asset.scene_id,

            transition: asset.transition,
            motion: asset.motion,

            visual_path: asset.visual_path,
            audio_path: asset.audio_path,

            duration: asset.duration,

            start_time,
            end_time,
        });

        cursor = end_time;
    }

    Ok(timelines)
}

pub async fn ffmpeg_render(target_path: String, timelines: &Vec<VideoTimeline>) -> Result<String, String> {
    let mut video_paths = vec![];
    
    for timeline in timelines {
        let video_path = format!("{}/videos/scene_{}.mp4", target_path, timeline.scene_id);

        // check file exists
        if !tokio::fs::try_exists(&video_path).await.unwrap_or(false) {
            render_scene_video(&timeline, &video_path).await?;
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

async fn generate_scene_visual(
    client: &Client,
    output_path: &str,
    prompt: &str,
) -> Result<(), String> {
    let image: Vec<u8> = cloudflare::generate_image(client, prompt).await?;
    
    if detect_media(&image.clone()).is_none() {
        return Err("Invalid visual response".to_string());
    }

    tokio::fs::write(output_path, image)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

async fn generate_scene_audio(
    client: &Client,
    output_path: &str,
    voice_mode: &VoiceMode,
    voice_segments: &Vec<VoiceSegment>,
) -> Result<(), String> {
    let mut audio_paths: Vec<String> = Vec::new();
            
    match voice_mode {
        VoiceMode::PerSegment => {
            for (i, segment) in voice_segments.iter().enumerate() {
                let audio = elevenlabs::generate_tts(
                    &client,
                    &segment.text,
                    Some(&segment.speaker),
                )
                .await?;

                if detect_media(&audio).is_none() {
                    return Err("Invalid audio response".to_string());
                }

                let segment_path = format!("{}.tmp_{}", output_path, i);

                tokio::fs::write(&segment_path, audio).await
                    .map_err(|e| e.to_string())?;

                audio_paths.push(segment_path);
            }
        }

        VoiceMode::SingleVoice => {
            let text = voice_segments
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

            tokio::fs::write(&output_path, audio).await
                .map_err(|e| e.to_string())?;
        }
    }

    if audio_paths.len() > 0 {
        let list_path = format!("{}.list", output_path);
    
        let list_content = audio_paths
            .iter()
            .map(|p| format!("file '{}'\n", p))
            .collect::<String>();

        tokio::fs::write(&list_path, list_content)
            .await
            .map_err(|e| e.to_string())?;

        let status = Command::new("ffmpeg")
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
                &output_path,
            ])
            .status()
            .await
            .map_err(|e| e.to_string())?;

        if !status.success() {
            return Err(format!("Failed to merge audios for scene {}", output_path));
        }
    }

    Ok(())
}

async fn render_scene_video(timeline: &VideoTimeline, output_path: &str) -> Result<(), String> {
    println!("🎬 [FFmpeg] Render scene starting");

    let visual_path = timeline.visual_path.clone();
    let audio_path = timeline.audio_path.clone();

    // 1. Check ffmpeg exists
    let ffmpeg_check = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .await;

    if ffmpeg_check.is_err() {
        return Err("ffmpeg not found in PATH. Please install ffmpeg or set PATH correctly.".to_string());
    }

    // 2. Check input files
    if !Path::new(&visual_path).exists() {
        return Err(format!("Missing visual file: {}", visual_path));
    }

    if !Path::new(&audio_path).exists() {
        return Err(format!("Missing audio file: {}", audio_path));
    }

    // 3. Create output directory
    if let Some(parent) = Path::new(output_path).parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create output dir: {}", e))?;
    }

    // 4. Run ffmpeg
    let duration = timeline.duration.to_string();
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-loop", "1",
            "-i", &visual_path,
            "-i", &audio_path,
            "-t", &duration,
            "-shortest",
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            output_path,
        ])
        .status()
        .await
        .map_err(|e| format!("Failed to execute ffmpeg: {}", e))?;

    if !status.success() {
        return Err(format!("ffmpeg failed with status: {}", status));
    }

    println!("✅ Scene rendered successfully: {}", output_path);

    Ok(())
}


async fn get_audio_duration(path: &str) -> anyhow::Result<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            path,
        ])
        .output()
        .await?;

    let duration = String::from_utf8(output.stdout)?
        .trim()
        .parse::<f64>()?;

    Ok(duration)
}