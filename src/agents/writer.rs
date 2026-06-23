use std::sync::Arc;
use async_trait::async_trait;

use crate::AppState;
use crate::agent::*;
use crate::enums::*;
use crate::models::*;
use crate::entities::*;
use crate::provider;


pub struct WriterAgent;

#[async_trait]
impl Agent for WriterAgent {
    async fn run(&self, state: &Arc<AppState>, job: &Job) -> Result<(), AgentError> {
        println!("✍️  [Writer] Generating video artifact...");
        
        let story_context: StoryContext =
            serde_json::from_str(&job.payload)
            .map_err(|e| AgentError::Decode(e.to_string()))?;

        let main_char =
            Character::find_char(&story_context.main_character)
            .unwrap_or(Character::main_char());
        let spotlight_chars =
            Character::find_chars(&story_context.spotlight_characters)
            .iter()
            .map(|c| {
                format!(
                    "- {}\n  Age: {}\n  Personality: {}\n  Role: {}\n  Relations: {}",
                    c.name(&state.config.movie.language),
                    c.age(),
                    c.personality_prompt,
                    c.role,
                    c.relationship_prompt
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let supporting_chars = Character::find_chars(&story_context.supporting_characters)
            .iter()
            .map(|c| {
                format!(
                    "- {}\n  Age: {}\n  Personality: {}\n  Role: {}\n  Relations: {}",
                    c.name(&state.config.movie.language),
                    c.age(),
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
                    c.name(&state.config.movie.language),
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
            main_char.visual_anchor(),
            spotlight_chars,
            supporting_chars,
            scene_motions,
            scene_transitions,
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
                - dialogue segments

            Return JSON only.
            "#,
            story_context.topic,
            story_context.main_character,
            story_context.spotlight_characters.join(", "),
            story_context.supporting_characters.join(", "),
            all_chars
        );

        let resp = self.execute(&state, &system, &prompt)
            .await
            .map_err(|e| AgentError::Execute(e.to_string()))?;

        let mut storyboard: Storyboard =
            serde_json::from_str(&resp)
            .map_err(|e| AgentError::Decode(e.to_string()))?;

        for scene in &mut storyboard.scenes {
            for segment in &mut scene.voice_segments {
                let voice_id = Character::find_char(&segment.speaker)
                    .and_then(|c| c.voice_id)
                    .map(str::to_owned)
                    .unwrap_or_else(|| state.config.voice.base_voice.clone());

                segment.voice_id = Some(voice_id);
            }
        }

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