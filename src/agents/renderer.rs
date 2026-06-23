
use std::sync::Arc;
use async_trait::async_trait;
use std::path;

use crate::AppState;
use crate::agent::*;
use crate::enums::*;
use crate::models::*;
use crate::entities::*;


pub struct RendererAgent;

#[async_trait]
impl Agent for RendererAgent {
    async fn run(&self, state: &Arc<AppState>, job: &Job) -> Result<(), AgentError> {
        println!("🎥 [Renderer] Rendering video...");
        
        let timeline: Timeline =
            serde_json::from_str(&job.payload)
            .map_err(|e| AgentError::Decode(e.to_string()))?;

        let video_metadata: VideoMetadata = self
            .execute(&timeline, job.workflow_path())
            .await
            .map_err(|e| AgentError::Execute(e.to_string()))?;

        let payload =
            serde_json::to_string(&video_metadata)
            .map_err(|e| AgentError::Encode(e.to_string()))?;

        state.services.db
            .handoff_job(job, AgentType::Publisher, payload)
            .await
            .map_err(|e| AgentError::Handoff(e.to_string()))?;

        Ok(())
    }
}

impl RendererAgent {
    async fn execute(
        &self,
        timeline: &Timeline,
        target_path: String
    ) -> Result<VideoMetadata, String> {
        if !std::process::Command::new("ffmpeg").arg("-version").output().is_ok() {
            return Err(
                "ffmpeg not found in PATH. Please install ffmpeg or set PATH correctly.".to_string()
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
                // println!("🎬 [FFmpeg] Starting render scene {}", clip.scene_id);

                // Check input files
                if !tokio::fs::try_exists(&clip.visual_path).await.unwrap_or(false) {
                    return Err(format!("Missing visual file: {}", clip.visual_path));
                }

                if !tokio::fs::try_exists(&clip.audio_path).await.unwrap_or(false) {
                    return Err(format!("Missing audio file: {}", clip.audio_path));
                }
                
                // Create output directory
                if let Some(parent) = path::Path::new(&video_path).parent() {
                    let _ = tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|e| e.to_string());
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
                    .map_err(|e| e.to_string())?;
                if !output.status.success() {
                    return Err(format!("FFmpeg failed: {}", String::from_utf8_lossy(&output.stderr)));
                }
            }

            video_paths.push(video_path);
        }
        
        println!("🎬 [FFmpeg] Starting render video {}", final_path);
        // println!("timeline.has_transition: {}", timeline.render_mode);

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

        // let full_cmd = std::iter::once(cmd.as_std().get_program().to_string_lossy().into_owned())
        //     .chain(cmd.as_std().get_args().map(|arg| arg.to_string_lossy().into_owned()))
        //     .collect::<Vec<_>>()
        //     .join(" ");
        //     println!("FFmpeg command:\n{}", full_cmd);

        let output = cmd
            .output()
            .await
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!("FFmpeg failed: {}", String::from_utf8_lossy(&output.stderr)));
        }
            
        println!("🎬 [FFmpeg] Rendered successfully");
        return Ok(VideoMetadata {
            title: timeline.title.clone(),
            video_path: final_path.to_string()
        });
    }
}