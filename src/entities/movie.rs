
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};


#[derive(Debug, Display, EnumString, Serialize, Deserialize, Clone)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Left,
    Center,
    Right,
}
impl Position {
    pub const ALL: &'static [Position] = &[
        Position::Left,
        Position::Center,
        Position::Right,
    ];
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

    pub fn ffmpeg_name(&self) -> &'static str {
        match self {
            Self::None => "fade",
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


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoMetadata {
    pub title: String,
    pub video_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Timeline {
    pub title: String,
    pub clips: Vec<Clip>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct StoryContext {
    pub topic: String,
    pub main_character: String,
    pub spotlight_characters: Vec<String>,
    pub supporting_characters: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Storyboard {
    pub title: String,
    pub shots: Vec<Shot>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Clip {
    pub shot_id: u32,
    pub audio_path: String,
    pub visual_path: String,
    pub video_path: String,
    pub subtitle_path: String,
    pub start_time: f64,
    pub end_time: f64,
    pub duration: f64,
    pub acrossfade: f64,
    pub transition: Transition,
    pub motion: Motion,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Shot {
    pub shot_id: u32,
    pub visual_prompt: String,
    pub motion: Motion,
    pub transition: Transition,
    pub actors: Vec<Actor>,
    pub dialogues: Vec<Dialogue>,
    pub environment: Environment,
    pub camera: Camera,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Actor {
    pub character: String,
    pub action: String,
    pub emotion: String,
    pub position: Position,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dialogue {
    pub character: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Camera {
    pub shot_type: String,   // or enum
    pub angle: String,
    pub movement: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Environment {
    pub location: String,
    pub time_of_day: String,
    pub weather: String,
}

impl Shot {
    pub fn visual_prompt(&self) -> String {
        let actors = if self.actors.is_empty() {
            "no characters".to_string()
        } else {
            self.actors
                .iter()
                .map(|a| a.character.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };

        let environment = format!(
            "{} at {}, {} weather",
            self.environment.location,
            self.environment.time_of_day,
            self.environment.weather
        );

        let camera = format!(
            "{} shot, {} angle, {} movement",
            self.camera.shot_type,
            self.camera.angle,
            self.camera.movement
        );

        format!(
            "{env}, actors: {actors}, camera: {camera}, cinematic lighting, ultra detailed, 9:16 vertical, realistic",
            env = environment,
            actors = actors,
            camera = camera
        )
    }
}