use std::path::Path;

use crate::AppContext;
use crate::api::*;
use crate::enums::*;
use crate::helper::*;
use crate::entities::*;


pub async fn build_content(
    ctx: &AppContext,
    system: &str,
    user: &str,
    is_json: bool,
) -> Result<String, ApiError> {
    let max_attempts = ctx.pm
        .key_count(Provider::Gemini)
        .await;

    for _ in 0..max_attempts {
        let guard = match ctx.pm.acquire(Provider::Gemini).await {
            Some(v) => v,
            None => break,
        };

        let provider = guard.provider.clone();
        let api_key = guard.credential.api_key.clone();

        let result = gemini::generate_script(
            &ctx.http,
            &api_key,
            system,
            user,
            is_json,
        )
        .await;

        match &result {
            Ok(_) => return result,

            Err(ApiError::QuotaExceeded(_, wait_sec)) => {
                ctx.pm
                    .block_key(
                        provider.clone(),
                        &api_key,
                        *wait_sec,
                    )
                    .await;

                continue;
            }

            Err(ApiError::RateLimited(_, wait_sec)) => {
                ctx.pm
                    .block_key(
                        provider.clone(),
                        &api_key,
                        *wait_sec,
                    )
                    .await;

                continue;
            }

            Err(_) => return result,
        }
    }

    Err(ApiError::QuotaExceeded(
        "All Gemini keys exhausted".into(),
        0,
    ))
}

pub async fn build_timeline(
    ctx: &AppContext,
    storyboard: &Storyboard,
    target_path: String,
    voice_mode: VoiceMode
) -> Result<Timeline, AppError> {
    let audio_root = format!("{}/audios", target_path);
    let visual_root = format!("{}/images", target_path);

    tokio::fs::create_dir_all(&visual_root)
        .await
        .map_err(|e| AppError::Build(e.to_string()))?;

    tokio::fs::create_dir_all(&audio_root)
        .await
        .map_err(|e| AppError::Build(e.to_string()))?;

    let mut handles = Vec::new();
    
    let sm = ctx.sm
        .get_or_create(Provider::ElevenLabs, 2)
        .await;

    for scene in storyboard.scenes.clone() {    
        let client = ctx.http.clone();
        let tts_setting = ctx.cfg.tts.clone();
        let diffusion_params = ctx.cfg.diffusion.clone();
        let voice_mode = voice_mode.clone();
        let audio_root = audio_root.clone();
        let visual_root = visual_root.clone();

        let pm: std::sync::Arc<crate::provider::ProviderManager> = ctx.pm.clone();
        let sm = sm.clone(); 

        handles.push(tokio::spawn(async move {
            let _permit = sm.acquire().await.unwrap();

            let visual_path = format!("{}/scene_{}.png", visual_root, scene.scene_id);
            let audio_path = format!("{}/scene_{}.mp3", audio_root, scene.scene_id);

            // ======================
            // VISUAL (Cloudflare)
            // ======================
            if !tokio::fs::try_exists(&visual_path).await.unwrap_or(false) {
                println!("🎬 [FFmpeg] Starting render visual {}", scene.scene_id);
               
                let guard = pm
                    .acquire(Provider::Cloudflare)
                    .await
                    .ok_or("No Cloudflare key")?;
                
                let provider = guard.provider.clone();
                let account_id = guard.credential.account_id.as_deref().unwrap().clone();
                let account_key = guard.credential.api_key.clone();

                let result = cloudflare::generate_image(
                    &client,
                    &account_id,
                    &account_key,
                    &scene.visual_prompt,
                    &diffusion_params,
                )
                .await;

                match result {
                    Ok(image) => {
                        if detect_media(&image).is_none() {
                            return Err("Invalid visual response".into());
                        }

                        tokio::fs::write(&visual_path, image)
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                    Err(e) => {
                        pm.block_key(
                            Provider::Cloudflare,
                            &account_key,
                            60,
                        )
                        .await;

                        match e {
                            ApiError::QuotaExceeded(_, wait_sec) => {
                                pm.block_key(provider.clone(), &account_key, wait_sec).await;
                            }
                            ApiError::RateLimited(_, wait_sec) => {
                                pm.block_key(provider.clone(), &account_key, wait_sec).await;
                            }
                            _ => {}
                        }

                        return Err(e.to_string());
                    }
                }
            }

            // ======================
            // AUDIO (ElevenLabs)
            // ======================
            if !tokio::fs::try_exists(&audio_path).await.unwrap_or(false) {
                println!("🎬 [FFmpeg] Starting render audio {}", scene.scene_id);                
                let mut audio_paths = Vec::new();
            
                match voice_mode {
                    VoiceMode::PerSegment => {
                        for (i, segment) in scene.voice_segments.iter().enumerate() {
                            let guard = pm
                                .acquire(Provider::ElevenLabs)
                                .await
                                .ok_or("No TTS key")?;

                            let provider = guard.provider.clone();
                            let api_key = guard.credential.api_key.clone();

                            let voice_id = segment
                                .voice_id
                                .as_deref()
                                .ok_or("Missing voice_id")?;

                            let result = elevenlabs::generate_tts(
                                &client,
                                &api_key,
                                &segment.text,
                                voice_id,
                                &tts_setting,
                            )
                            .await;

                            match result {
                                Ok(audio) => {
                                    if detect_media(&audio).is_none() {
                                        return Err("Invalid audio response".to_string());
                                    }

                                    let segment_path = format!("{}.tmp_{}", audio_path, i);

                                    tokio::fs::write(&segment_path, audio).await
                                        .map_err(|e| e.to_string())?;

                                    audio_paths.push(segment_path);
                                },

                                Err(e) => {
                                    pm.block_key(
                                        Provider::ElevenLabs,
                                        &api_key,
                                        60,
                                    )
                                    .await;

                                    match e {
                                        ApiError::QuotaExceeded(_, wait_sec) => {
                                            pm.block_key(provider.clone(), &api_key, wait_sec).await;
                                        }
                                        ApiError::RateLimited(_, wait_sec) => {
                                            pm.block_key(provider.clone(), &api_key, wait_sec).await;
                                        }
                                        _ => {}
                                    }

                                    return Err(e.to_string());
                                }
                            }
                        }
                    }

                    VoiceMode::SingleVoice => {
                        let guard = pm
                            .acquire(Provider::ElevenLabs)
                            .await
                            .ok_or("No TTS key")?;

                        let api_key = guard.credential.api_key.clone();
                        
                        let text =
                            scene.voice_segments
                            .iter()
                            .map(|v| v.text.as_str())
                            .collect::<Vec<_>>()
                            .join("\n");

                        let first_segment =
                            scene.voice_segments
                            .first()
                            .ok_or_else(|| "voice_segments is empty".to_string())?;

                        let voice_id =
                            first_segment
                            .voice_id
                            .as_deref()
                            .ok_or_else(|| "Missing voice_id in first segment".to_string())?;


                        let result = elevenlabs::generate_tts(
                            &client,
                            &api_key,
                            &text,
                            &voice_id,
                            &tts_setting
                        )
                        .await;

                        match result {
                            Ok(audio) => {
                                if detect_media(&audio).is_none() {
                                    return Err("Invalid audio response".to_string());
                                }

                                tokio::fs::write(&audio_path, audio)
                                    .await
                                    .map_err(|e| e.to_string())?;
                            },

                            Err(e) => {
                                pm.block_key(
                                    Provider::ElevenLabs,
                                    &api_key,
                                    60,
                                )
                                .await;

                                return Err(e.to_string());
                            }
                        };
                    }
                }

                if audio_paths.len() > 0 {
                    let list_path = format!("{}.list", audio_path);
                
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
                            "-f", "concat",
                            "-safe", "0",
                            "-i", &list_path,
                            "-c", "copy",
                            &audio_path,
                        ])
                        .status()
                        .await
                        .map_err(|e| e.to_string())?;

                    if !status.success() {
                        return Err(format!("Failed to merge audios for scene {}", audio_path));
                    }
                }
            }


            let duration = get_audio_duration(&audio_path).await.map_err(|e| e.to_string())?;

            Ok::<_, String>(Clip {
                scene_id: scene.scene_id,
                transition: scene.transition,
                motion: scene.motion,
                audio_path,
                visual_path,
                duration,
                acrossfade: 0.0,
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
                .map_err(|e| AppError::Build(e.to_string()))?
                .map_err(|e| AppError::Build(e.to_string()))?
        );
    }

    assets.sort_by_key(|v| v.scene_id);

    let mut cursor = 0.0;
    let mut clips = Vec::new();
  
    let has_transition = assets
        .iter()
        .any(|c| c.transition.is_active());
    let render_mode = match has_transition {
        true => RenderMode::FilterComplex,
        _ => RenderMode::Concat,
    };
    let transition_duration = match render_mode {
        RenderMode::Concat => 0.0,
        RenderMode::FilterComplex => Transition::DURATION,
    };

    for asset in assets {
        let start_time = cursor;
        let end_time = start_time + asset.duration + transition_duration;

        clips.push(Clip {
            scene_id: asset.scene_id,
            transition: asset.transition,
            motion: asset.motion,
            visual_path: asset.visual_path,
            audio_path: asset.audio_path,
            duration: asset.duration + transition_duration,
            acrossfade: transition_duration,
            start_time,
            end_time,
        });

        cursor = end_time;
    }

    Ok(Timeline {
        title: storyboard.title.clone(),
        render_mode,
        clips,
    })
}

pub async fn ffmpeg_render(
    timeline: &Timeline,
    target_path: String
) -> Result<VideoMetadata, AppError> {
    if !ffmpeg_check() {
        return Err(
            AppError::Build(
                "ffmpeg not found in PATH. Please install ffmpeg or set PATH correctly.".to_string()
            )
        );
    }
    
    let final_path = format!("{}/final_video.mp4", &target_path);
    let mut video_paths = vec![];
    
    for clip in timeline.clips.iter() {
        let video_path = format!("{}/videos/scene_{}.mp4", target_path, clip.scene_id);

        // check file exists
        if tokio::fs::try_exists(&video_path).await.unwrap_or(false) {
            println!("🎬 [FFmpeg] Scene {} already rendered", clip.scene_id);
        } else {
            println!("🎬 [FFmpeg] Starting render scene {}", clip.scene_id);

            // Check input files
            if !tokio::fs::try_exists(&clip.visual_path).await.unwrap_or(false) {
                return Err(
                    AppError::Build(
                        format!("Missing visual file: {}", clip.visual_path)
                    )
                )
            }

            if !tokio::fs::try_exists(&clip.audio_path).await.unwrap_or(false) {
                return Err(
                    AppError::Build(
                        format!("Missing audio file: {}", clip.audio_path)
                    )
                )
            }
            
            // Create output directory
            if let Some(parent) = Path::new(&video_path).parent() {
                let _ = tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| AppError::Build(e.to_string()));
            }

            // Run ffmpeg
            let mut cmd = tokio::process::Command::new("ffmpeg");
            let duration = clip.duration.to_string();

            cmd.args([
                "-y",
                "-framerate", "25",
                "-loop", "1",
                "-i", &clip.visual_path,
                "-i", &clip.audio_path,
                "-t", &duration,
            ]);
            if let Some(filter) = clip.motion.ffmpeg_filter(clip.duration) {
                cmd.args([
                    "-vf", &filter,
                ]);
            }
            if let Some(filter_complex) = clip.transition.ffmpeg_filter(clip.acrossfade) {
                cmd.args([
                    "-filter_complex", &filter_complex,
                    "-map", "0:v",
                    "-map", "[aout]",
                ]);
            }
            cmd.args([
                // "-shortest",
                "-c:v", "libx264",
                "-pix_fmt", "yuv420p",
                "-c:a", "aac",
                &video_path,
            ]);

            let output = cmd
                .output()
                .await
                .map_err(|e| AppError::Build(e.to_string()))?;
            if !output.status.success() {
                return Err(
                    AppError::Build(
                        format!("FFmpeg failed: {}", String::from_utf8_lossy(&output.stderr))
                    )
                );
            }
        }

        video_paths.push(video_path);
    }
    
    println!("🎬 [FFmpeg] Starting render video {}", final_path);
    println!("timeline.has_transition: {}", timeline.render_mode);

    let mut cmd = tokio::process::Command::new("ffmpeg");
    cmd.arg("-y");
    for video_path in &video_paths {
        cmd.args(["-i", video_path]);
    }

    match timeline.render_mode {
        RenderMode::Concat => {
            let mut filter_parts = String::new();
            let mut filter_lables = String::new();

            for i in 0..video_paths.len() {
                filter_lables.push_str(&format!("[{}:v][{}:a]", i, i));
            }

            filter_parts.push_str(&format!(
                "{}concat=n={}:v=1:a=1[v][a]",
                filter_lables,
                video_paths.len()
            ));

            cmd.args([
                "-filter_complex", &filter_parts,
                "-map", "[v]",
                "-map", "[a]",
            ]);
        }
        RenderMode::FilterComplex => {
            let mut filter_parts = Vec::new();
            let mut offset_frames = timeline.clips[0].duration - timeline.clips[0].acrossfade;

            // Video chain
            for i in 1..timeline.clips.len() {
                if i == 1 {
                    filter_parts.push(format!(
                        "[0:v][1:v]xfade=transition={}:duration={}:offset={}[v1]",
                        timeline.clips[i - 1].transition.ffmpeg_name(),
                        timeline.clips[i].acrossfade,
                        offset_frames
                    ));
                } else {
                    filter_parts.push(format!(
                        "[v{}][{}:v]xfade=transition={}:duration={}:offset={}[v{}]",
                        i - 1,
                        i,
                        timeline.clips[i - 1].transition.ffmpeg_name(),
                        timeline.clips[i].acrossfade,
                        offset_frames,
                        i
                    ));
                }

                offset_frames += timeline.clips[i].duration - timeline.clips[i].acrossfade;
            }

            // Audio chain
            for i in 1..timeline.clips.len() {
                if i == 1 {
                    filter_parts.push(format!(
                        "[0:a][1:a]acrossfade=d={}[a1]",
                        timeline.clips[i].acrossfade
                    ));
                } else {
                    filter_parts.push(format!(
                        "[a{}][{}:a]acrossfade=d={}[a{}]",
                        i - 1,
                        i,
                        timeline.clips[i].acrossfade,
                        i
                    ));
                }
            }

            cmd.args([
                "-filter_complex", &filter_parts.join(";"),
                "-map", &format!("[v{}]", timeline.clips.len() - 1),
                "-map", &format!("[a{}]", timeline.clips.len() - 1),
            ]);
        }
    }
    cmd.args([
        "-c:v", "libx264",
        "-c:a", "aac",
        "-movflags", "+faststart",
        "-pix_fmt", "yuv420p",
        &final_path,
    ]);

    let full_cmd = std::iter::once(cmd.as_std().get_program().to_string_lossy().into_owned())
        .chain(cmd.as_std().get_args().map(|arg| arg.to_string_lossy().into_owned()))
        .collect::<Vec<_>>()
        .join(" ");
        println!("FFmpeg command:\n{}", full_cmd);

    let output = cmd
        .output()
        .await
        .map_err(|e| AppError::Build(e.to_string()))?;
    if !output.status.success() {
        return Err(
            AppError::Build(
                format!("FFmpeg failed: {}", String::from_utf8_lossy(&output.stderr))
            )
        );
    }
        
    println!("🎬 [FFmpeg] Rendered successfully");
    return Ok(VideoMetadata {
        title: timeline.title.clone(),
        video_path: final_path.to_string()
    });
}

fn ffmpeg_check() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok()
}

async fn _merge_audio_files(
    inputs: &[String],
    output: &str,
) -> Result<(), String> {
    // placeholder ffmpeg merge logic
    let mut combined = Vec::new();

    for path in inputs {
        let data = tokio::fs::read(path)
            .await
            .map_err(|e| e.to_string())?;

        combined.extend(data);
    }

    tokio::fs::write(output, combined)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn _get_scene_duration(video_path: &str, audio_path: &str) -> Result<f64, String> {
    let v = _get_video_duration(video_path).await?;
    let a = get_audio_duration(audio_path).await?;

    // debug
    println!("video_duration={:.6}, audio_duration={:.6}", v, a);

    // chọn video làm master, nhưng fallback nếu lỗi
    let duration = if v > 0.0 { v } else { a };

    // optional safety: tránh lệch quá lớn
    if (a - v).abs() > 0.2 {
        println!("⚠️ warning: audio/video drift detected");
    }

    Ok(duration)
}

pub async fn _get_video_duration(path: &str) -> Result<f64, String> {
    let output = tokio::process::Command::new("ffprobe")
        .args([
            "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            path,
        ])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    let s = String::from_utf8_lossy(&output.stdout);
    s.trim().parse::<f64>().map_err(|e| e.to_string())
}

async fn get_audio_duration(path: &str) -> Result<f64, String> {
    let output = tokio::process::Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            path,
        ])
        .output()
        .await
        .map_err(|e| e.to_string())?;

    
    let s = String::from_utf8_lossy(&output.stdout);
    s.trim().parse::<f64>().map_err(|e| e.to_string())
}