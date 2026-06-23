use std::sync::Arc;
use async_trait::async_trait;

use crate::AppState;
use crate::agent::*;
use crate::models::*;
use crate::entities::*;
use crate::provider;


pub struct PlannerAgent;

#[async_trait]
impl Agent for PlannerAgent {
    async fn run(&self, state: &Arc<AppState>, job: &Job) -> Result<(), AgentError> {
        println!("🧠 [Planner] Creating topic...");

        let main_char = Character::main_char();
        let spotlight_chars = Character::spotlight_chars()
            .iter()
            .map(|c| format!(
                "- {} ({})\n  Age: {}\n Personality: {}\n  Relations: {}",
                c.name(&state.config.movie.language),
                c.topic_domain,
                c.age(),
                c.personality_prompt,
                c.relationship_prompt
            ))
            .collect::<Vec<_>>()
            .join("\n");

        let relation_chars = Character::relation_chars()
            .iter()
            .map(|c| format!(
                "- {} ({})\n  Age: {}\n Role: {}\n  Relations: {}",
                c.name(&state.config.movie.language),
                c.profession,
                c.age(),
                c.role,
                c.relationship_prompt
            ))
            .collect::<Vec<_>>()
            .join("\n");
        
        // Extract the history and pass it to the Task.
        let history_list = state.services.db
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
            state.config.movie.language,
        );

        let prompt = format!(r#"
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
        

        let resp = self
            .execute(&state, &system, &prompt)
            .await
            .map_err(|e| AgentError::Execute(e.to_string()))?;

        let story_context: StoryContext =
            serde_json::from_str(&resp)
            .map_err(|e| AgentError::Encode(e.to_string()))?;

        let payload =
            serde_json::to_string(&story_context)
                .map_err(|e| AgentError::Decode(e.to_string()))?;

        state.services.db
            .handoff_job(job, AgentType::Writer, payload)
            .await
            .map_err(|e| AgentError::Handoff(e.to_string()))?;

        state.services.db
            .save_topic(&job.workflow_id, story_context.topic.clone())
            .await
            .map_err(|e| AgentError::Handoff(e.to_string()))?;

        Ok(())
    }
}

impl PlannerAgent {
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
                return Err(format!("{} is acquired", &provider.to_string()));
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