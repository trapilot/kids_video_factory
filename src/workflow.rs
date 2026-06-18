use reqwest::Client;
use serde_json::Value;
use std::collections::HashSet;

use crate::config::CONFIG;
use crate::enums::*;
use crate::models::*;
use crate::renderer::*;
use crate::helper::*;
use crate::db::DbManager;
use crate::uploader;


pub async fn run_agent_workflow(
    client: &Client,
    db: &DbManager,
    state: &mut VideoState
) -> Result<VideoArtifact, String> {
        println!(
            "▶️ Run workflow | node: {:?} | retry: {} | topic: {}",
            state.current_node,
            state.meta.retry_count,
            state.target_topic,
        );

        loop {
            match state.current_node {
                AgentNode::Planner => {
                    println!("🧠[Planner] Creating topic...");
                    let spotlight_chars = EDU_CHARACTERS
                        .iter()
                        .filter(|c| c.enabled && c.spotlight)
                        .map(|c| format!(
                            "- {} ({})\n  Age: {}\n Personality: {}\n  Relations: {}",
                            c.name,
                            c.topic_domain,
                            now_age(c.birth_year),
                            c.personality_prompt,
                            c.relationship_prompt
                        ))
                        .collect::<Vec<_>>()
                        .join("\n");

                    let relation_chars = EDU_CHARACTERS
                        .iter()
                        .filter(|c| c.enabled && !c.spotlight)
                        .map(|c| format!(
                            "- {} ({})\n  Age: {}\n Role: {}\n  Relations: {}",
                            c.name,
                            c.profession,
                            now_age(c.birth_year),
                            c.role,
                            c.relationship_prompt
                        ))
                        .collect::<Vec<_>>()
                        .join("\n");
                    
                    // Extract the history and pass it to the Task.
                    let history_list = db.get_recent_topics(state.target_age, 20)
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
                        CONFIG.movie.language,
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
                    
                    let resp = build_content(client, &system, &user, true).await?;
                    let parsed: Value = serde_json::from_str(&resp).map_err(|e| ctx("JSON parse", e))?;
                    
                    state.target_topic = parsed["topic"].as_str().unwrap().to_string();
                    state.main_character = parsed["main_character"].as_str().unwrap().to_string();
                    state.spotlight_characters =  parsed["spotlight_characters"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().unwrap().to_string())
                        .collect();
                    state.supporting_characters =  parsed["supporting_characters"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_str().unwrap().to_string())
                        .collect();
                    
                    state.current_node = AgentNode::Writer;
                }

                AgentNode::Writer => {
                    println!("✍️ [Writer] Generating final video artifact...");
                    
                    let main_char = EDU_CHARACTERS
                        .iter()
                        .find(|c| c.name == state.main_character)
                        .unwrap();

                    let spotlight_set: HashSet<&str> = state.spotlight_characters
                        .iter()
                        .map(|s| s.as_str())
                        .collect();
                    let spotlight_chars = EDU_CHARACTERS
                        .iter()
                        .filter(|c| spotlight_set.contains(c.name))
                        .map(|c| {
                            format!(
                                "- {}\n  Age: {}\n  Personality: {}\n  Role: {}\n  Relations: {}",
                                c.name,
                                now_age(c.birth_year),
                                c.personality_prompt,
                                c.role,
                                c.relationship_prompt
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    let supporting_set: HashSet<&str> = state.supporting_characters
                        .iter()
                        .map(|s| s.as_str())
                        .collect();
                    let supporting_chars = EDU_CHARACTERS
                        .iter()
                        .filter(|c| supporting_set.contains(c.name))
                        .map(|c| {
                            format!(
                                "- {}\n  Age: {}\n  Personality: {}\n  Role: {}\n  Relations: {}",
                                c.name,
                                now_age(c.birth_year),
                                c.personality_prompt,
                                c.role,
                                c.relationship_prompt
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    let all_chars = EDU_CHARACTERS
                        .iter()
                        .map(|c| {
                            format!(
                                "- {}: {}",
                                c.name,
                                c.personality_prompt
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    
                    let scene_motions = Motion::ALL
                        .iter()
                        .map(|m| format!("- {}", m))
                        .collect::<Vec<_>>()
                        .join("\n");

                    let scene_transitions = Transition::ALL
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
                        main_char.visual_anchor,
                        spotlight_chars,
                        supporting_chars,
                        scene_motions,
                        scene_transitions,
                        CONFIG.movie.language,
                        CONFIG.movie.language,
                    );

                    let user = format!(r#"
                        TOPIC: {}

                        MAIN CHARACTER:
                        {}

                        SUPPORTING CHARACTERS:
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
                        state.target_topic,
                        state.main_character,
                        state.spotlight_characters.join(", "),
                        state.supporting_characters.join(", "),
                        all_chars
                    );

                    let resp = build_content(client, &system, &user, true).await?;

                    let mut artifact: VideoArtifact = serde_json::from_str(&resp)
                        .map_err(|e| ctx("JSON parse", e))?;

                    for scene in &mut artifact.scenes {
                        for segment in &mut scene.voice_segments {
                            segment.voice_id = Some(
                                EDU_CHARACTERS
                                .iter()
                                .find(|c| c.name == segment.speaker)
                                .map(|c| c.voice_id.to_string())
                                .unwrap_or(CONFIG.voice.base_voice.to_string())
                            );
                        }
                    }

                    state.final_json = Some(artifact);
                    state.current_node = AgentNode::Builder;
                }

                AgentNode::Builder => {
                    println!("🔧[Builder] Generating assets...");
                    let artifact = state.final_json.as_ref().unwrap();

                    let timelines = build_timelines(
                        &client,
                        state.target_path.clone(),
                        artifact.scenes.clone(),
                        VoiceMode::SingleVoice,
                    ).await?;

                    state.video_timelines = timelines;
                    state.current_node = AgentNode::Renderer;
                }

                AgentNode::Renderer => {
                    println!("🎥 [Renderer] Rendering video...");
                    
                    let final_video = ffmpeg_render(
                        state.target_path.clone(),
                        &state.video_timelines,
                    ).await?;

                    state.video_path = final_video;
                    state.current_node = AgentNode::Publisher;
                }

                AgentNode::Publisher => {
                    println!("📤 [Publisher] Publishing the video...");
                    
                    let (yt_res, tt_res) = tokio::join!(
                        uploader::upload_to_youtube(&client, &state.video_path, &state.target_topic),
                        uploader::upload_to_tiktok(&client, &state.video_path, &state.target_topic),
                    );
                    
                    state.youtube_uploaded = yt_res.is_ok();
                    state.tiktok_uploaded = tt_res.is_ok();

                    if !state.youtube_uploaded {
                        if let Err(e) = yt_res {
                            eprintln!("🔴 YouTube error: {}", e);
                        }
                    }
                    if !state.tiktok_uploaded {
                        if let Err(e) = tt_res {
                            eprintln!("🔴 Tiktok error: {}", e);
                        }
                    }

                    if !state.youtube_uploaded {
                        return Err("Upload failed".to_string());
                    }
                    
                    state.current_node = AgentNode::End;
                }

                AgentNode::End => {
                    return Ok(state.final_json.clone().unwrap());
                }
            };
        }
}