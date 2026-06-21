
use async_trait::async_trait;

use crate::agent::*;
use crate::enums::*;
use crate::models::*;
use crate::entities::*;
use crate::workflow;
use crate::provider;


pub struct BuilderAgent;

#[async_trait]
impl Agent for BuilderAgent {
    async fn run(&self,  ctx: &workflow::Context, job: &Job) -> Result<(), AgentError> {
        println!("🔧[Builder] Generating assets...");
        
        let storyboard: Storyboard =
            serde_json::from_str(&job.payload)
            .map_err(|e| AgentError::Decode(e.to_string()))?;

        let timeline: Timeline = self.execute(
                &ctx,
                &storyboard,
                job.workflow_path(),
                VoiceMode::SingleVoice
            )
            .await
            .map_err(|e| AgentError::BuildTimeline(e.to_string()))?;

        let payload = serde_json::to_string(&timeline)
            .map_err(|e| AgentError::Encode(e.to_string()))?;

        ctx.db
            .handoff_job(job, AgentType::Renderer, payload)
            .await
            .map_err(|e| AgentError::Handoff(e.to_string()))?;

        Ok(())
    }
}

impl BuilderAgent {
    async fn execute(
        &self,
        ctx: &workflow::Context,
        storyboard: &Storyboard,
        target_path: String,
        voice_mode: VoiceMode
    ) -> Result<Timeline, String> {
        let audio_root = format!("{}/audios", target_path);
        let visual_root = format!("{}/images", target_path);

        tokio::fs::create_dir_all(&visual_root)
            .await
            .map_err(|e| e.to_string())?;

        tokio::fs::create_dir_all(&audio_root)
            .await
            .map_err(|e| e.to_string())?;

        let mut handles = Vec::new();

        let base_voice = &ctx.cfg.voice.base_voice;
        let pm = &ctx.pm;

        for scene in storyboard.scenes.clone() {
            let voice_mode = voice_mode.clone();
            let audio_root = audio_root.clone();
            let visual_root = visual_root.clone();

            let pm = pm.clone(); 
            let base_voice = base_voice.clone();

            handles.push(tokio::spawn(async move {
                let visual_path = format!("{}/scene_{}.png", visual_root, scene.scene_id);
                let audio_path = format!("{}/scene_{}.mp3", audio_root, scene.scene_id);

                // ======================
                // VISUAL (Cloudflare)
                // ======================
                if !tokio::fs::try_exists(&visual_path).await.unwrap_or(false) {
                    println!("🎬 [FFmpeg] Starting render visual {}", scene.scene_id);

                    let req = provider::ProviderRequest::Image(provider::ImageRequest {
                        prompt: scene.visual_prompt,
                        num_steps: Some(4),
                        guidance: Some(3.5), 
                        width: None,
                        height: None,
                    });

                    let provider = provider::Provider::CFWorker;
                    let guard = match pm.acquire(&provider).await {
                        Some(v) => v,
                        None => {
                            return Err(format!("Provider not found: {}", &provider.to_string()));
                        }
                    };
                    
                    let rsp = guard.clone()
                        .call(req)
                        .await
                        .map_err(|e| e.to_string())?
                        .into_bytes()
                        .map_err(|e| e.to_string());

                    match rsp {
                        Ok(image) => {
                            tokio::fs::write(&visual_path, image)
                                .await
                                .map_err(|e| e.to_string())?;
                            println!("🎬 [FFmpeg] Render visual {} successful", scene.scene_id);
                        }
                        Err(e) => {
                            println!("🎬 [FFmpeg] Render visual {} failed", scene.scene_id);
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
                    
                    let provider = provider::Provider::ElevenLabs;
                    let guard = match pm.acquire(&provider).await {
                        Some(v) => v,
                        None => {
                            return Err(format!("Provider not found: {}", &provider.to_string()));
                        }
                    };

                    match voice_mode {
                        VoiceMode::PerSegment => {
                            for (i, segment) in scene.voice_segments.iter().enumerate() {
                                let req = provider::ProviderRequest::Audio(provider::AudioRequest {
                                    text: segment.text.clone(),
                                    voice_id: segment.voice_id.clone(),
                                    language: None,
                                    speed: None,
                                    stability: None,
                                    similarity_boost: None,
                                    format: Some(provider::AudioFormat::Wav),
                                });

                                let rsp = guard.clone()
                                    .call(req)
                                    .await
                                    .map_err(|e| e.to_string())?
                                    .into_bytes()
                                    .map_err(|e| e.to_string());

                                match rsp {
                                    Ok(audio) => {
                                        let segment_path = format!("{}.tmp_{}", audio_path, i);

                                        tokio::fs::write(&segment_path, audio)
                                            .await
                                            .map_err(|e| e.to_string())?;

                                        audio_paths.push(segment_path);
                                    },

                                    Err(e) => {
                                        return Err(e.to_string());
                                    }
                                }
                            }
                        }

                        VoiceMode::SingleVoice => {
                            let text =
                                scene.voice_segments
                                .iter()
                                .map(|v| v.text.as_str())
                                .collect::<Vec<_>>()
                                .join("\n");

                            let req = provider::ProviderRequest::Audio(provider::AudioRequest {
                                text: text,
                                voice_id: Some(base_voice),
                                language: None,
                                speed: None,
                                stability: None,
                                similarity_boost: None,
                                format: Some(provider::AudioFormat::Wav),
                            });

                            let rsp = guard.clone()
                                .call(req)
                                .await
                                .map_err(|e| e.to_string())?
                                .into_bytes()
                                .map_err(|e| e.to_string());

                            match rsp {
                                Ok(audio) => {
                                    tokio::fs::write(&audio_path, audio)
                                        .await
                                        .map_err(|e| e.to_string())?;
                                },

                                Err(e) => {
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

                let duration = Self.get_audio_duration(&audio_path)
                    .await
                    .map_err(|e| e.to_string())?;

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
                    .map_err(|e| e.to_string())?
                    .map_err(|e| e.to_string())?
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

    async fn get_audio_duration(&self, path: &str) -> Result<f64, String> {
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
}