use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};
#[derive(Debug)]

pub enum AppError {
    Provider(ApiError),
    Database(String),
    Json(String),
    Build(String),
    Upload(String),
}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Provider(err) => {
                write!(f, "Provider Error: {}", err)
            }

            AppError::Upload(err) => {
                write!(f, "Upload Error: {}", err)
            }

            AppError::Database(msg) => {
                write!(f, "Database Error: {}", msg)
            }

            AppError::Json(msg) => {
                write!(f, "Json Error: {}", msg)
            }

            AppError::Build(msg) => {
                write!(f, "Build Error: {}", msg)
            }
        }
    }
}
impl std::error::Error for AppError {}
impl From<ApiError> for AppError {
    fn from(err: ApiError) -> Self {
        AppError::Provider(err)
    }
}

#[derive(Debug)]
pub enum ApiError {
    Unauthorized(String),
    QuotaExceeded(String, u64),
    RateLimited(String, u64),
    InvalidResponse(String),
    InvalidApiKey(String),
    Internal(String),
    Network(String),
    NotSupport(String),
}
impl From<ApiError> for String {
    fn from(err: ApiError) -> Self {
        err.to_string()
    }
}
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Unauthorized(msg) => {
                write!(f, "Unauthorized: {}", msg)
            }

            ApiError::QuotaExceeded(msg, wait) => {
                write!(f, "Quota exceeded: {}s {}", wait, msg)
            }

            ApiError::RateLimited(msg, wait) => {
                write!(f, "Rate limited: {}s {}", wait, msg)
            }

            ApiError::InvalidApiKey(msg) => {
                write!(f, "Invalid API key: {}", msg)
            }

            ApiError::InvalidResponse(msg) => {
                write!(f, "Invalid response: {}", msg)
            }

            ApiError::NotSupport(msg) => {
                write!(f, "Not supported: {}", msg)
            }

            _ => write!(f, "Invalid ApiError")
        }
    }
}

#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize, Eq, PartialEq, Hash, sqlx::Type)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase", type_name = "text")]
pub enum AgentType {
    Manager,
    Planner,
    Writer,
    Builder,
    Renderer,
    Publisher,
    Cleaner,
}
impl Default for AgentType {
    fn default() -> Self {
        AgentType::Manager
    }
}

#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize, sqlx::Type)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase", type_name = "text")]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize, sqlx::Type)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase", type_name = "text")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}
impl Default for JobStatus {
    fn default() -> Self {
        JobStatus::Pending
    }
}

#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Gemini,
    Cloudflare,
    ElevenLabs,
}

#[derive(Debug, Clone)]
pub enum MediaType {
    Image,
    Video,
    Audio,
}

#[derive(Debug, Clone, Display, EnumString)]
pub enum VoiceMode {
    PerSegment,
    SingleVoice,
}

#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RenderMode {
    Concat,
    FilterComplex,
}

#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    None,
    ZoomIn,
    ZoomOut,
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
    KenBurns,
    DollyIn,
    DollyOut,
}
impl Motion {
    pub const ALL: &'static [Motion] = &[
        Motion::None,
        Motion::ZoomIn,
        Motion::ZoomOut,
        Motion::PanLeft,
        Motion::PanRight,
        Motion::PanUp,
        Motion::PanDown,
        Motion::KenBurns,
        Motion::DollyIn,
        Motion::DollyOut,
    ];

    pub fn ffmpeg_filter(&self, duration_secs: f64) -> Option<String> {
        let frames = (duration_secs * 25.0) as u32;

        match self {
            Motion::None => None,
            Motion::ZoomIn => Some(format!("zoompan=z='min(zoom+0.001,1.3)':d={}", frames)),
            Motion::ZoomOut => Some(format!("zoompan=z='max(1.0,1.3-on*0.001)':d={}", frames)),
            Motion::PanLeft => Some(format!("zoompan=x='iw/2-(on*2)':z=1.1:d={}", frames)),
            Motion::PanRight => Some(format!("zoompan=x='on*2':z=1.1:d={}", frames)),
            Motion::PanUp => Some(format!("zoompan=y='ih/2-(on*2)':z=1.1:d={}", frames)),
            Motion::PanDown => Some(format!("zoompan=y='on*2':z=1.1:d={}", frames)),
            Motion::KenBurns => Some(format!("zoompan=z='min(zoom+0.0005,1.2)':x='on':y='on':d={}", frames)),
            Motion::DollyIn => Some(format!("zoompan=z='min(zoom+0.001,1.5)':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d={}", frames)),
            Motion::DollyOut => Some(format!("zoompan=z='max(1.0,1.5-on*0.002)':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d={}", frames)),
        }
    }
}

#[derive(Debug, Clone, Display, EnumString, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Transition {
    None,
    Fade,
    SlideLeft,
    SlideRight,
    WipeLeft,
    WipeRight,
    CircleOpen,
    FadeBlack,
    FadeWhite,
}
impl Transition {
    pub const DURATION: f64 = 2.0;
    pub const ALL: &'static [Transition] = &[
        Transition::None,
        Transition::Fade,
        Transition::SlideLeft,
        Transition::SlideRight,
        Transition::WipeLeft,
        Transition::WipeRight,
        Transition::CircleOpen,
        Transition::FadeBlack,
        Transition::FadeWhite,
    ];

    pub fn is_active(&self) -> bool {
        !matches!(self, Transition::None)
    }

    pub fn ffmpeg_name(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Fade => "fade",
            Self::SlideLeft => "slideleft",
            Self::SlideRight => "slideright",
            Self::WipeLeft => "wipeleft",
            Self::WipeRight => "wiperight",
            Self::CircleOpen => "circleopen",
            Self::FadeBlack => "fadeblack",
            Self::FadeWhite => "fadewhite",
        }
    }
    
    pub fn ffmpeg_filter(&self, duration_secs: f64) -> Option<String> {
        let adelay_ms = (duration_secs / 2.0) * 1000.0;
        match self {
            _ => Some(format!("[1:a]adelay={0}|{0}[a_delayed];[a_delayed]apad[aout]", adelay_ms)),
        }
    }
}
