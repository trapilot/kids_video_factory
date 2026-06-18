use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use reqwest::Client;

use crate::api::*;
use crate::config::CONFIG;
use crate::enums::*;
use crate::helper::*;
use crate::models::*;

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
    let tts_semaphore = Arc::new(tokio::sync::Semaphore::new(2)); // ElevenLabs free max 2 concurrent

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
    if !ffmpeg_check() {
        return Err("ffmpeg not found in PATH. Please install ffmpeg or set PATH correctly.".to_string());
    }
    
    let final_path = format!("{}/final_video.mp4", &target_path);
    let final_stream = format!("[v{}]", timelines.len() - 1);
    
    if tokio::fs::try_exists(&final_path).await.unwrap_or(false) {
        println!("🎬 FFmpeg already rendered");
        return Ok(final_path.to_string());
    }

    let mut video_paths = vec![];
    
    for timeline in timelines {
        let video_path = format!("{}/videos/scene_{}.mp4", target_path, timeline.scene_id);

        // check file exists
        if tokio::fs::try_exists(&video_path).await.unwrap_or(false) {
            println!("🎬 [FFmpeg] Scene {} already rendered", timeline.scene_id);
        } else {
            println!("🎬 [FFmpeg] Starting render scene {}", timeline.scene_id);

            // Check input files
            if tokio::fs::try_exists(&timeline.visual_path).await.unwrap_or(false) {
                return Err(format!("Missing visual file: {}", timeline.visual_path));
            }

            if tokio::fs::try_exists(&timeline.audio_path).await.unwrap_or(false) {
                return Err(format!("Missing audio file: {}", timeline.audio_path));
            }

            // Create output directory
            if let Some(parent) = Path::new(&video_path).parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("Failed to create output dir: {}", e))?;
            }

            // Run ffmpeg
            let mut cmd = tokio::process::Command::new("ffmpeg");
            let duration = timeline.duration.to_string();

            cmd.args([
                "-y",
                "-loop", "1",
                "-i", &timeline.visual_path,
                "-i", &timeline.audio_path,
                "-t", &duration,
            ]);
            if let Ok(motion) = Motion::from_str(&timeline.motion) {
                if let Some(filter) = motion.ffmpeg_filter(timeline.duration) {
                    cmd.args([
                        "-vf", &filter,
                    ]);
                }
            }
            cmd.args([
                "-shortest",
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                &video_path,
            ]);

            let output = cmd
                .output()
                .await
                .map_err(|e| e.to_string())?;
            
            if !output.status.success() {
                return Err(format!(
                    "FFmpeg failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }

        video_paths.push(video_path);
    }
    
    println!("🎬 [FFmpeg] Rendering the final: {}", final_path);
    let mut filter_parts = Vec::new();
    let mut offset = timelines[0].duration - Transition::DURATION;
    for i in 1..timelines.len() {
        let transition =Transition::from_str(&timelines[i - 1].transition)
                .unwrap_or(Transition::DEFAULT);

        if i == 1 {
            filter_parts.push(format!(
                "[0:v][1:v]xfade=transition={}:duration={}:offset={}[v1]",
                transition.ffmpeg_name(),
                Transition::DURATION,
                offset
            ));
        } else {
            filter_parts.push(format!(
                "[v{}][{}:v]xfade=transition={}:duration={}:offset={}[v{}]",
                i - 1,
                i,
                transition.ffmpeg_name(),
                Transition::DURATION,
                offset,
                i
            ));
        }

        offset += timelines[i].duration - Transition::DURATION;
    }

    let mut cmd = tokio::process::Command::new("ffmpeg");
    cmd.arg("-y");
    for video_path in &video_paths {
        cmd.args(["-i", video_path]);
    }
    cmd.args([
        "-filter_complex", &filter_parts.join(";"),
    ]);
    cmd.args([
        "-map", &final_stream,
        "-c:v", "libx264",
        &final_path,
    ]);
    
    println!("🎬 [FFmpeg] Rendered successfully");
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
                    segment.voice_id.as_deref().unwrap_or(CONFIG.voice.base_voice),
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
                &voice_segments.first().unwrap().voice_id.as_deref().unwrap_or(CONFIG.voice.base_voice),
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

fn ffmpeg_check() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok()
}

async fn get_audio_duration(path: &str) -> anyhow::Result<f64> {
    let output = tokio::process::Command::new("ffprobe")
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