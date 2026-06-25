
use std::sync::Arc;
use async_trait::async_trait;
use futures::StreamExt;
use futures::TryStreamExt;
use hound::{WavReader, WavWriter};

use crate::AppState;
use crate::agent::*;
use crate::models::*;
use crate::entities::*;
use crate::provider;
use crate::config;


#[derive(Debug, Clone)]
pub struct ShotAssets {
    pub root: String,
    pub image: String,
    pub audio: String,
    pub video: String,
    pub subtitle: String,
}

pub struct RenderedShot {
    pub shot_id: u32,
    pub image_path: String,
    pub audio_path: String,
    pub video_path: String,
    pub subtitle_path: String,
    pub duration: f64,
    pub motion: Motion,
    pub transition: Transition,
}

impl ShotAssets {
    pub fn new(base: &str, shot_id: u32) -> Self {
        let root = format!("{base}/shot_{shot_id}");

        Self {
            image: format!("{root}/image.png"),
            audio: format!("{root}/audio.wav"),
            video: format!("{root}/video.mp4"),
            subtitle: format!("{root}/subtitle.ass"),
            root,
        }
    }
}

pub struct BuilderAgent;

#[async_trait]
impl Agent for BuilderAgent {
    async fn run(&self, state: &Arc<AppState>, job: &Job) -> Result<(), AgentError> {
        println!("🔧 [Builder] Generating assets...");
        
        let storyboard: Storyboard =
            serde_json::from_str(&job.payload)
            .map_err(|e| AgentError::Decode(e.to_string()))?;

        let timeline: Timeline = self.execute(
                &state,
                &storyboard,
                job.workflow_path(),
            )
            .await
            .map_err(|e| AgentError::Execute(e.to_string()))?;

        let payload = serde_json::to_string(&timeline)
            .map_err(|e| AgentError::Encode(e.to_string()))?;

        state.services.db
            .handoff_job(job, AgentType::Renderer, payload)
            .await
            .map_err(|e| AgentError::Handoff(e.to_string()))?;

        Ok(())
    }
}

impl BuilderAgent {
    async fn execute(
        &self,
        state: &Arc<AppState>,
        storyboard: &Storyboard,
        target_path: String
    ) -> Result<Timeline, String> {
        let handles = storyboard
            .shots
            .iter()
            .cloned()
            .filter(|shot| !shot.dialogues.is_empty())
            .map(|shot| {
                let providers = state.services.providers.clone();
                let config = state.config.clone();
                let target_path = target_path.clone();

                tokio::spawn(async move {
                    let assets = ShotAssets::new(&target_path, shot.shot_id);
                    let _ = tokio::fs::create_dir_all(&assets.root)
                            .await
                            .map_err(|e| e.to_string());

                    let exists_image = tokio::fs::try_exists(&assets.image).await.unwrap_or(false);
                    let exists_audio = tokio::fs::try_exists(&assets.audio).await.unwrap_or(false);
                    
                    if !exists_image {
                        let _ = text_to_image(
                            shot.visual_prompt(),
                            assets.clone(),
                            config.clone(),
                            providers.clone()
                        ).await;
                    }

                    if !exists_audio {
                        let _ = text_to_audio(
                            shot.dialogues.clone(),
                            assets.clone(),
                            config.clone(),
                            providers.clone()
                        ).await;
                    }
                    
                    let duration = get_audio_duration(&assets.audio).await?;

                    Ok::<_, String>(RenderedShot {
                        shot_id: shot.shot_id,
                        image_path: assets.image,
                        audio_path: assets.audio,
                        video_path: assets.video,
                        subtitle_path: assets.subtitle,
                        duration: (duration + 0.05).ceil(),
                        transition: shot.transition,
                        motion: shot.motion,
                    })
                })
            })
            .collect::<Vec<_>>();
        

        let mut rendered_shots = Vec::new();

        for handle in handles {
            let shot = handle
                .await
                .map_err(|e| e.to_string())??;

            rendered_shots.push(shot);
        }

        rendered_shots.sort_by_key(|v| v.shot_id);
        
        let mut cursor = 0.0;
        let mut clips = Vec::new();
    
        let transition_duration = Transition::DURATION;
        for shot in rendered_shots {
            let start_time = cursor;

            let end_time = start_time + shot.duration + transition_duration;

            clips.push(Clip {
                shot_id: shot.shot_id,
                visual_path: shot.image_path,
                audio_path: shot.audio_path,
                video_path: shot.video_path,
                subtitle_path: shot.subtitle_path,
                duration: shot.duration + transition_duration,
                start_time,
                end_time,
                acrossfade: transition_duration,
                transition: shot.transition,
                motion: shot.motion,
            });

            cursor = end_time;
        }

        Ok(Timeline {
            title: storyboard.title.clone(),
            clips,
        })
    }
}

async fn text_to_image(
    prompt: String,
    assets: ShotAssets,
    config: Arc<config::Config>,
    providers: Arc<provider::ProviderManager>,
) -> Result<(), String> {
    let req =
        provider::ProviderRequest::Image(
            provider::ImageRequest {
                prompt: prompt,
                num_steps: Some(config.diffusion.num_steps),
                guidance: Some(config.diffusion.guidance),
                width: None,
                height: None,
            },
        );

    let provider = provider::Provider::CFWorker;

    let guard = providers
        .acquire(&provider)
        .await
        .ok_or_else(|| {
            format!("Provider not found")
        })?;

    let image = guard
        .call(req)
        .await
        .map_err(|e| e.to_string())?
        .into_bytes()
        .map_err(|e| e.to_string())?;

    tokio::fs::write(&assets.image, image)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}


async fn text_to_audio(
    dialogues: Vec<Dialogue>,
    assets: ShotAssets,
    _config: Arc<config::Config>,
    _providers: Arc<provider::ProviderManager>,
) -> Result<(), String> {
    const MODEL: &str = "./tools/models/en_US-lessac-medium.onnx";
    const PIPER_BIN: &str = "./tools/piper/piper";
    const PIPER_CONCURRENCY: usize = 4;

    // println!("cwd: {:?}", std::env::current_dir());
    // println!("piper exists: {}", std::path::Path::new(PIPER_BIN).exists());
    // println!("model exists: {}", std::path::Path::new(MODEL).exists());

    let final_path = assets.audio.clone();
    let root_path = assets.root.clone();

    let mut audio_paths: Vec<(usize, String)> =
        futures::stream::iter(dialogues.into_iter().enumerate())
            .map(|(i, dialogue)| {
                let text = dialogue.text.clone();
                let segment_path = format!("{}/audio.part_{i}.wav", root_path);

                async move {
                    let mut child = tokio::process::Command::new(PIPER_BIN)
                        .args([
                            "--model", MODEL,
                            "--output_file",
                            &segment_path,
                        ])
                        .stdin(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .spawn()
                        .map_err(|e| format!("Piper spawn failed: {e}"))?;

                    if let Some(mut stdin) = child.stdin.take() {
                        tokio::io::AsyncWriteExt::write_all(
                            &mut stdin,
                            text.as_bytes(),
                        )
                        .await
                        .map_err(|e| format!("Piper stdin failed: {e}"))?;
                    }

                    let output = child
                        .wait_with_output()
                        .await
                        .map_err(|e| format!("Piper wait failed: {e}"))?;

                    if !output.status.success() {
                        return Err(format!(
                            "Piper synthesis failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        ));
                    }

                    Ok::<_, String>((i, segment_path))
                }
            })
            .buffer_unordered(PIPER_CONCURRENCY)
            .try_collect()
            .await?;

    if audio_paths.is_empty() {
        let silent_path = format!("{}/audio.part_0.wav", root_path);

        let status = tokio::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-f", "lavfi",
                "-i", "anullsrc=r=44100:cl=stereo",
                "-t", "5",
                "-acodec", "pcm_s16le",
                &final_path,
            ])
            .status()
            .await
            .map_err(|e| format!("Create silent audio failed: {e}"))?;

        if !status.success() {
            return Err("Failed to create silent audio".into());
        }

        audio_paths.push((0, silent_path));
    }

    audio_paths.sort_by_key(|(i, _)| *i);
    
    let first = hound::WavReader::open(&audio_paths[0].1)
        .map_err(|e| e.to_string())?;

    let spec = first.spec();

    let mut writer = hound::WavWriter::create(
        &final_path,
        spec,
    )
    .map_err(|e| e.to_string())?;

    for (_, path) in audio_paths {
        let mut reader =
            hound::WavReader::open(path)
                .map_err(|e| e.to_string())?;

        for sample in reader.samples::<i16>() {
            writer
                .write_sample(
                    sample.map_err(|e| e.to_string())?
                )
                .map_err(|e| e.to_string())?;
        }
    }

    writer.finalize()
        .map_err(|e| e.to_string())?;

    Ok(())
        
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
    s.trim().parse::<f64>()
        .map_err(|e| format!("Parse duration {} --> {}", path, e.to_string()))
}