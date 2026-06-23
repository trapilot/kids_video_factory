use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Config {
    pub movie: MovieConfig,
    pub voice: VoiceConfig,
    pub tts: TtsConfig,
    pub diffusion: DiffusionParams,
    pub feature: FeatureConfig,
    pub workflow_per_day: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MovieConfig {
    pub language: String,
    pub country: String,
    pub default_description: String,
    pub default_tags: Vec<String>,
    pub youtube_category: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VoiceConfig {
    pub base_voice: String,
    pub default_gender: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TtsConfig {
    pub speed: f32,
    pub stability: f32,
    pub similarity_boost: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiffusionParams {
    pub num_steps: u32,
    pub guidance: f32
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeatureConfig {
    pub enable_cache: bool,
    pub enable_subtitle: bool,
    pub enable_translation: bool,
}

pub fn build_config() -> Config {
    Config {
        workflow_per_day: 1,
        movie: MovieConfig {
            language: "English".to_string(),
            country: "Vietnam".to_string(),
            default_description: "An AI-generated educational short movie".to_string(),
            default_tags: vec![
                "ai movie".to_string(),
                "english".to_string(),
                "vietnamese".to_string(),
                "education".to_string(),
                "children".to_string(),
            ],
            youtube_category: "27".to_string(),
        },
        voice: VoiceConfig {
            base_voice: "pNInz6obpgDQGcFmaJgB".to_string(),
            default_gender: "female".to_string(),
        },
        tts: TtsConfig {
            speed: 1.0,
            stability: 0.8,
            similarity_boost: 0.75,
        },
        diffusion: DiffusionParams {
            num_steps: 4,
            guidance: 3.5, 
        },
        feature: FeatureConfig {
            enable_cache: false,
            enable_subtitle: false,
            enable_translation: false,
        },
    }
}