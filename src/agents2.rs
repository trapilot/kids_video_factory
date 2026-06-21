
use async_trait::async_trait;

use crate::agent;
use crate::enums;
use crate::models;
use crate::errors;
use crate::entities;
use crate::producer;
use crate::uploader;
use crate::workflow;


pub struct ManagerAgent;
pub struct PlannerAgent;
pub struct WriterAgent;
pub struct BuilderAgent;
pub struct RendererAgent;
pub struct PublisherAgent;
pub struct CleanerAgent;


#[async_trait]
impl agent::Agent for ManagerAgent {
    async fn run(&self, _ctx: &workflow::Context, job: &models::Job) -> Result<(), errors::AgentError> {
        println!("Running job: {}", job.id);
        Ok(())
    }
}

#[async_trait]
impl agent::Agent for PlannerAgent {
    async fn run(&self, ctx: &workflow::Context, job: &models::Job) -> Result<(), errors::AgentError> {
        println!("🧠[Planner] Creating topic...");

        let main_char = entities::Character::main_char();
        let spotlight_chars = entities::Character::spotlight_chars()
            .iter()
            .map(|c| format!(
                "- {} ({})\n  Age: {}\n Personality: {}\n  Relations: {}",
                c.name(&ctx.cfg.movie.language),
                c.topic_domain,
                c.age(),
                c.personality_prompt,
                c.relationship_prompt
            ))
            .collect::<Vec<_>>()
            .join("\n");

        let relation_chars = entities::Character::relation_chars()
            .iter()
            .map(|c| format!(
                "- {} ({})\n  Age: {}\n Role: {}\n  Relations: {}",
                c.name(&ctx.cfg.movie.language),
                c.profession,
                c.age(),
                c.role,
                c.relationship_prompt
            ))
            .collect::<Vec<_>>()
            .join("\n");
        
        // Extract the history and pass it to the Task.
        let history_list = ctx.db
            .get_recent_topics(main_char.age(), 7)
            .await
            .unwrap_or_default();


        let system = format!(r#"
            You are the Content Director for a children's animated series.
            Returns JSON:
            {{
                "topic": "...",
                "main_character": "...",
                "spotlight_characters": ["...", "..."],
                "supporting_characters": ["...", "..."]
            }}
            Rules:
            1. Use only {}.
            2. Main Character must come from Spotlight Characters.
            3. Spotlight characters must come from Spotlight Characters, depending on what suits the story.
            3. Supporting characters must come from Relation Characters, depending on what suits the story.
            4. Only the characters provided may be used.
            5. Theme must fit the main character.
            6. Suitable for children aged in Spotlight Characters.
            7. Avoid repeating previous topics.
            "#,
            ctx.cfg.movie.language,
        );

        let user = format!(r#"
            SPOTLIGHT CHARACTERS:
            {}

            RELATION CHARACTERS:
            {}

            PREVIOUS TOPICS:
            {}

            Tasks:
            1. Choose a spotlight character.
            2. Create a fresh educational topic.
            3. Select spotlight and supporting characters if needed.
            4. Returns JSON in the correct format.
            "#,
            spotlight_chars,
            relation_chars,
            history_list.is_empty()
                .then(|| "Not yet".to_string())
                .unwrap_or_else(|| history_list.join(", "))
        );
        

        let resp =
            producer::build_content(&ctx, &system, &user, true)
            .await
            .map_err(|e| errors::AgentError::Invalid(e.to_string()))?;

        let story_context: entities::StoryContext =
            serde_json::from_str(&resp)
            .map_err(|e| errors::AgentError::Encode(e.to_string()))?;

        let payload =
            serde_json::to_string(&story_context)
                .map_err(|e| errors::AgentError::Decode(e.to_string()))?;

        ctx.db
            .handoff_job(job, agent::AgentType::Writer, payload)
            .await
            .map_err(|e| errors::AgentError::Database(e.to_string()))?;

        ctx.db
            .save_topic(&job.workflow_id, story_context.topic.clone())
            .await
            .map_err(|e| errors::AgentError::Database(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl agent::Agent for WriterAgent {
    async fn run(&self, ctx: &workflow::Context, job: &models::Job) -> Result<(), errors::AgentError> {
        println!("✍️ [Writer] Generating video artifact...");
        
        let story_context: entities::StoryContext =
            serde_json::from_str(&job.payload)
            .map_err(|e| errors::AgentError::Decode(e.to_string()))?;

        let main_char =
            entities::Character::find_char(&story_context.main_character)
            .unwrap_or(entities::Character::main_char());
        let spotlight_chars =
            entities::Character::find_chars(&story_context.spotlight_characters)
            .iter()
            .map(|c| {
                format!(
                    "- {}\n  Age: {}\n  Personality: {}\n  Role: {}\n  Relations: {}",
                    c.name(&ctx.cfg.movie.language),
                    c.age(),
                    c.personality_prompt,
                    c.role,
                    c.relationship_prompt
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let supporting_chars = entities::Character::find_chars(&story_context.supporting_characters)
            .iter()
            .map(|c| {
                format!(
                    "- {}\n  Age: {}\n  Personality: {}\n  Role: {}\n  Relations: {}",
                    c.name(&ctx.cfg.movie.language),
                    c.age(),
                    c.personality_prompt,
                    c.role,
                    c.relationship_prompt
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let all_chars = entities::EDU_CHARACTERS
            .iter()
            .map(|c| {
                format!(
                    "- {}: {}",
                    c.name(&ctx.cfg.movie.language),
                    c.personality_prompt
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        let scene_motions = enums::Motion::ALL
            .iter()
            .map(|m| format!("- {}", m))
            .collect::<Vec<_>>()
            .join("\n");

        let scene_transitions = enums::Transition::ALL
            .iter()
            .map(|m| format!("- {}", m))
            .collect::<Vec<_>>()
            .join("\n");

        let system = format!(r#"
            You are a senior children's animation writer, create the FINAL JSON artifact.

            MAIN CHARACTER:
            {}

            VISUAL STYLE PREFIX:
            {}

            SPOTLIGHT CHARACTERS:
            {}

            SUPPORTING CHARACTERS:
            {}

            MOTION MUST BE ONE OF:
            {}

            TRANSITION MUST BE ONE OF:
            {}

            Motion guidelines:
            - Dialogue scene -> ZoomIn
            - Walking scene -> PanLeft or PanRight
            - Landscape scene -> KenBurns
            - Emotional moment -> ZoomIn
            - Ending scene -> ZoomOut

            Transition guidelines:
            - Same location -> Fade
            - New location -> SlideLeft
            - Time skip -> FadeBlack
            - Important reveal -> CircleOpen

            Rules:
            1. Entire content must be {}.
            2. Suitable for Main Character age.
            3. Video length: 45-60 seconds.
            4. Story must have 3-5 scenes.
            5. Every scene must contain dialogue.
            6. Use only provided characters.
            7. Main character must drive the story.
            8. Visual prompts must be kid-friendly.
            9. No Chinese elements.
            10. Use {} context only.
            11. Never invent new motion names.
            12. Never invent new transition names.

            Return STRICT JSON:
            {{
            "title": "...",
            "scenes": [
                {{
                "scene_id": 1,
                "duration": 5,
                "transition": "...",
                "motion": "...",
                "visual_prompt": "...",
                "voice_segments": [
                    {{
                        "speaker": "...",
                        "text": "..."
                    }}
                ]
                }}
            ]
            }}
            "#,
            main_char.personality_prompt,
            main_char.visual_anchor(),
            spotlight_chars,
            supporting_chars,
            scene_motions,
            scene_transitions,
            ctx.cfg.movie.language,
            ctx.cfg.movie.language,
        );

        let user = format!(r#"
            TOPIC: {}

            MAIN CHARACTER:
            {}

            SPOTLIGHT CHARACTERS:
            {}

            SUPPORTING CHARACTERS:
            {}

            ALL CHARACTER PERSONALITIES:
            {}

            Create:
                - complete story
                - scene breakdown
                - visual prompts
                - dialogue segments

            Return JSON only.
            "#,
            story_context.topic,
            story_context.main_character,
            story_context.spotlight_characters.join(", "),
            story_context.supporting_characters.join(", "),
            all_chars
        );

        let resp =
            producer::build_content(&ctx, &system, &user, true)
            .await
            .map_err(|e| errors::AgentError::Invalid(e.to_string()))?;

        let mut storyboard: entities::Storyboard =
            serde_json::from_str(&resp)
            .map_err(|e| errors::AgentError::Decode(e.to_string()))?;

        for scene in &mut storyboard.scenes {
            for segment in &mut scene.voice_segments {
                let voice_id = entities::Character::find_char(&segment.speaker)
                    .and_then(|c| c.voice_id)
                    .map(str::to_owned)
                    .unwrap_or_else(|| ctx.cfg.voice.base_voice.clone());

                segment.voice_id = Some(voice_id);
            }
        }

        let payload =
            serde_json::to_string(&storyboard)
            .map_err(|e| errors::AgentError::Encode(e.to_string()))?;

        ctx.db
            .handoff_job(job, agent::AgentType::Builder, payload)
            .await
            .map_err(|e| errors::AgentError::Database(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl agent::Agent for BuilderAgent {
    async fn run(&self,  ctx: &workflow::Context, job: &models::Job) -> Result<(), errors::AgentError> {
        println!("🔧[Builder] Generating assets...");
        
        let storyboard: entities::Storyboard =
            serde_json::from_str(&job.payload)
            .map_err(|e| errors::AgentError::Decode(e.to_string()))?;

        let timeline: entities::Timeline =
            producer::build_timeline(&ctx, &storyboard, job.workflow_path(), enums::VoiceMode::SingleVoice)
            .await
            .map_err(|e| errors::AgentError::Invalid(e.to_string()))?;

        let payload = serde_json::to_string(&timeline)
            .map_err(|e| errors::AgentError::Encode(e.to_string()))?;

        ctx.db
            .handoff_job(job, agent::AgentType::Renderer, payload)
            .await
            .map_err(|e| errors::AgentError::Database(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl agent::Agent for RendererAgent {
    async fn run(&self, ctx: &workflow::Context, job: &models::Job) -> Result<(), errors::AgentError> {
        println!("🎥 [Renderer] Rendering video...");
        
        let timeline: entities::Timeline =
            serde_json::from_str(&job.payload)
            .map_err(|e| errors::AgentError::Decode(e.to_string()))?;

        let video_metadata: entities::VideoMetadata =
            producer::ffmpeg_render(&timeline, job.workflow_path())
            .await
            .map_err(|e| errors::AgentError::Invalid(e.to_string()))?;

        let payload =
            serde_json::to_string(&video_metadata)
            .map_err(|e| errors::AgentError::Encode(e.to_string()))?;

        ctx.db
            .handoff_job(job, agent::AgentType::Renderer, payload)
            .await
            .map_err(|e| errors::AgentError::Database(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl agent::Agent for PublisherAgent {
    async fn run(&self,  ctx: &workflow::Context, job: &models::Job) -> Result<(), errors::AgentError> {
        println!("📤 [Publisher] Publishing the video...");
        
        let video_metadata: entities::VideoMetadata =
            serde_json::from_str(&job.payload)
            .map_err(|e| errors::AgentError::Decode(e.to_string()))?;

        let default_settings = ctx.cfg.movie.clone();

        let (yt_res, tt_res) = tokio::join!(
            uploader::upload_to_youtube(&ctx, &video_metadata.video_path, entities::YoutubePayload {
                title: video_metadata.title.clone(),
                description: default_settings.default_description,
                tags: default_settings.default_tags,
                category_id: default_settings.youtube_category,
            }),
            uploader::upload_to_tiktok(&ctx, &video_metadata.video_path, entities::TiktokPayload {
                title: video_metadata.title.clone(),
                privacy_level: "PUBLIC_TO_EVERYONE".to_string(),
                disable_comment: true,
            }),
        );
        
        let publish_state = entities::PublishState {
            all_uploaded: yt_res.is_ok() && tt_res.is_ok(),
            any_uploaded: yt_res.is_ok() ^ tt_res.is_ok(),
            errors: entities::PublishError {
                youtube: yt_res.err().map(|e| e.to_string()),
                tiktok: tt_res.err().map(|e| e.to_string()),
            }
        };

        if !publish_state.any_uploaded {
            if let Some(err) = &publish_state.errors.youtube {
                return Err(errors::AgentError::Upload(err.to_string()));
            }

            if let Some(err) = &publish_state.errors.tiktok {
                return Err(errors::AgentError::Upload(err.to_string()));
            }
        }

        let payload =
            serde_json::to_string(&publish_state)
            .map_err(|e| errors::AgentError::Encode(e.to_string()))?;

        // ctx.db
        //     .handoff_job(job, agent::AgentType::Cleaner, payload)
        //     .await
        //     .map_err(|e| errors::AgentError::Database(e.to_string()))?;

        ctx.db
            .complete_job(&job.id, payload)
            .await
            .map_err(|e| errors::AgentError::Database(e.to_string()))?;
        
        Ok(())
    }
}

#[async_trait]
impl agent::Agent for CleanerAgent {
    async fn run(&self,  ctx: &workflow::Context, job: &models::Job) -> Result<(), errors::AgentError> {
        println!("📤 [Cleaner] Cleaning the video...");

        let publish_state: entities::PublishState =
            serde_json::from_str(&job.payload)
            .map_err(|e| errors::AgentError::Decode(e.to_string()))?;
        
        if let Some(err) = &publish_state.errors.youtube {
            eprintln!("🔴 YouTube error: {}", err);
        }

        if let Some(err) = &publish_state.errors.tiktok {
            eprintln!("🔴 TikTok error: {}", err);
        }

        ctx.db
            .complete_job(&job.id, "DONE".to_string())
            .await
            .map_err(|e| errors::AgentError::Database(e.to_string()))?;

        Ok(())
    }
}