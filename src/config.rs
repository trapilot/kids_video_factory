use serde::Deserialize;

#[derive(Debug)]
pub struct Config {
    pub movie: MovieConfig,
    pub voice: VoiceConfig,
    pub tts: TtsConfig,
    pub feature: FeatureConfig,
}

#[derive(Debug, Deserialize)]
pub struct MovieConfig {
    pub language: &'static str,
    pub country: &'static str,
    pub default_description: &'static str,
    pub default_tags: [&'static str; 5],
    pub youtube_category: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct VoiceConfig {
    pub base_voice: &'static str,
    pub default_gender: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct TtsConfig {
    pub speed: f32,
    pub stability: f32,
    pub similarity_boost: f32,
}

#[derive(Debug, Deserialize)]
pub struct FeatureConfig {
    pub enable_cache: bool,
    pub enable_subtitle: bool,
    pub enable_translation: bool,
}

pub static CONFIG: Config = Config {
    movie: MovieConfig {
        language: "English", // Vietnamese
        country: "Vietnam",
        default_description: "An AI-generated educational short movie",
        default_tags: ["ai movie", "english", "vietnamese", "education", "children"],
        youtube_category: "27",
    },
    
    voice: VoiceConfig {
        base_voice: "pNInz6obpgDQGcFmaJgB",
        default_gender: "female",
    },

    tts: TtsConfig {
        speed: 1.0,
        stability: 0.8,
        similarity_boost: 0.75,

    },

    feature: FeatureConfig {
        enable_cache: false,
        enable_subtitle: false,
        enable_translation: false,
    }
};
