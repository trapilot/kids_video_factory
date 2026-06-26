use std::sync::Arc;
use async_trait::async_trait;

use crate::AppState;
use crate::agent::*;
use crate::models::*;
use crate::entities::*;
use crate::provider;


pub struct WriterAgent;

#[async_trait]
impl Agent for WriterAgent {
    async fn run(&self, state: &Arc<AppState>, job: &Job) -> Result<(), AgentError> {
        println!("✍️  [Writer] Generating artifact...");
        
        let story_context: StoryContext =
            serde_json::from_str(&job.payload)
            .map_err(|e| AgentError::Decode(e.to_string()))?;
        
        let all_chars = EDU_CHARACTERS
            .iter()
            .map(|c| {
                format!(
                    "- {}\n  Age: {}\n  Role: {}\n  Personality: {}\n  Relations: {}\n  Visual Style: {}",
                    c.name(&state.config.movie.language),
                    c.age(),
                    c.role,
                    c.personality_prompt,
                    c.relationship_prompt,
                    c.visual_anchor()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        let all_motions = Motion::ALL
            .iter()
            .map(|m| format!("- {}", m))
            .collect::<Vec<_>>()
            .join("\n");

        let all_transitions = Transition::ALL
            .iter()
            .map(|m| format!("- {}", m))
            .collect::<Vec<_>>()
            .join("\n");

        let all_positions = Position::ALL
            .iter()
            .map(|m| format!("- {}", m))
            .collect::<Vec<_>>()
            .join("\n");

        let system = format!(r#"
            You are an expert film storyboard artist and AI video director.

            Your task is to convert the provided script into a sequence of SHORT VIDEO SHOTS suitable for TikTok, Reels, and YouTube Shorts.

            MOTION MUST BE ONE OF:
            {}

            TRANSITION MUST BE ONE OF:
            {}

            ACTOR POSITION MUST BE ONE OF:
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
            2. Each shot should represent only ONE visual idea.
            3. Each shot should contain approximately 1-2 spoken sentences.
            4. Keep each narration segment short enough to be spoken in roughly 3-5 seconds.
            5. If a narration is too long, split it into multiple shots.
            6. Maintain visual consistency of characters across all shots.
            7. Every shot must have a detailed visual_prompt suitable for image generation.
            8. Describe what each character is doing and feeling.
            9. Camera movement should be simple and realistic.
            10. No Chinese elements.
            11. Use {} context only.

            Return STRICT JSON ONLY:
            {{
                "title": "...",
                "characters": [
                    {{
                        "name": "...",
                        "appearance": "...",
                        "voice": "..."
                    }}
                ],
                "shots": [
                    {{
                        "shot_id": 1,
                        "transition": "...",
                        "motion": "...",
                        "environment":
                            {{
                                "location": "...",
                                "time_of_day": "...",
                                "weather": "..."
                            }},
                        "camera":
                            {{
                                "shot_type": "close_up | medium | wide",
                                "angle": "front | low | high | side",
                                "movement": "static | zoom_in | zoom_out | pan_left | pan_right"
                            }},
                        "visual_prompt": "string,
                        "actors": [
                            {{
                                "character": "...",
                                "position": "...",
                                "action": "...",
                                "emotion": "..."
                            }}
                        ],
                        "dialogues": [
                            {{
                                "character": "...",
                                "text": "..."
                            }}
                        ]
                    }}
                ]
                }}
            "#,
            all_motions,
            all_transitions,
            all_positions,
            state.config.movie.language,
            state.config.movie.language,
        );

        let prompt = format!(r#"
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
                - actors MUST NOT empty
                - dialogues MUST NOT empty

            Return JSON only.
            "#,
            story_context.topic,
            story_context.main_character,
            story_context.spotlight_characters.join(", "),
            story_context.supporting_characters.join(", "),
            all_chars,
        );

        let resp = self.execute(&state, &system, &prompt)
            .await
            .map_err(|e| AgentError::Execute(e.to_string()))?;

        let storyboard: Storyboard =
            serde_json::from_str(&resp)
            .map_err(|e| AgentError::Decode(e.to_string()))?;

        let payload =
            serde_json::to_string(&storyboard)
            .map_err(|e| AgentError::Encode(e.to_string()))?;

        state.services.db
            .handoff_job(job, AgentType::Builder, payload)
            .await
            .map_err(|e| AgentError::Handoff(e.to_string()))?;

        Ok(())
    }
}

impl WriterAgent {
    async fn execute(
        &self,
        state: &Arc<AppState>,
        system: &str,
        prompt: &str,
    ) -> Result<String, String> {
        let provider = &provider::Provider::Gemini;
        let guard = match state.services.providers.acquire(&provider).await {
            Some(v) => v,
            None => {
                return Err(format!("{}", &provider.to_string()));
            }
        };

        let req = provider::ProviderRequest::Script(provider::ScriptRequest {
            system: system.to_string(),
            prompt: prompt.to_string(),
            json_mode: true,
            temperature: None,
            max_tokens: None,
        });

        let rsp = guard
            .call(req)
            .await
            .map_err(|e| e.to_string())?;

        rsp
            .into_text()
            .map_err(|e| e.to_string())
        
    }
}