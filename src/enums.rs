
use std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentNode {
    Planner,
    Writer,
    Builder,
    Renderer,
    Publisher,
    End,
}
impl Default for AgentNode {
    fn default() -> Self {
        AgentNode::Planner
    }
}
impl std::fmt::Display for AgentNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentNode::Planner => write!(f, "planner"),
            AgentNode::Writer => write!(f, "writer"),
            AgentNode::Builder => write!(f, "builder"),
            AgentNode::Renderer => write!(f, "renderer"),
            AgentNode::Publisher => write!(f, "publisher"),
            AgentNode::End => write!(f, "end"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MediaType {
    Image,
    Video,
    Audio,
}

#[derive(Debug, Clone)]
pub enum VoiceMode {
    PerSegment,
    SingleVoice,
}


#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
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

    pub fn ffmpeg_filter(&self, duration: f64) -> Option<String> {
        let frames = (duration * 25.0) as u32;

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
impl std::fmt::Display for Motion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Motion::None => write!(f, "None"),
            Motion::ZoomIn => write!(f, "ZoomIn"),
            Motion::ZoomOut => write!(f, "ZoomOut"),
            Motion::PanLeft => write!(f, "PanLeft"),
            Motion::PanRight => write!(f, "PanRight"),
            Motion::PanUp => write!(f, "PanUp"),
            Motion::PanDown => write!(f, "PanDown"),
            Motion::KenBurns => write!(f, "KenBurns"),
            Motion::DollyIn => write!(f, "DollyIn"),
            Motion::DollyOut => write!(f, "DollyOut"),
        }
    }
}
impl FromStr for Motion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "None" => Ok(Motion::None),
            "ZoomIn" => Ok(Motion::ZoomIn),
            "ZoomOut" => Ok(Motion::ZoomOut),
            "PanLeft" => Ok(Motion::PanLeft),
            "PanRight" => Ok(Motion::PanRight),
            "PanUp" => Ok(Motion::PanUp),
            "PanDown" => Ok(Motion::PanDown),
            "KenBurns" => Ok(Motion::KenBurns),
            "DollyIn" => Ok(Motion::DollyIn),
            "DollyOut" => Ok(Motion::DollyOut),
            _ => Err(format!("Invalid motion: {}", s)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
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
    pub const DURATION: f64 = 1.0;
    pub const DEFAULT: Transition = Transition::Fade;
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
}
impl std::fmt::Display for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transition::None => write!(f, "None"),
            Transition::Fade => write!(f, "Fade"),
            Transition::SlideLeft => write!(f, "SlideLeft"),
            Transition::SlideRight => write!(f, "SlideRight"),
            Transition::WipeLeft => write!(f, "WipeLeft"),
            Transition::WipeRight => write!(f, "WipeRight"),
            Transition::CircleOpen => write!(f, "CircleOpen"),
            Transition::FadeBlack => write!(f, "FadeBlack"),
            Transition::FadeWhite => write!(f, "FadeWhite"),
        }
    }
}
impl FromStr for Transition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "None" => Ok(Transition::None),
            "Fade" => Ok(Transition::Fade),
            "SlideLeft" => Ok(Transition::SlideLeft),
            "SlideRight" => Ok(Transition::SlideRight),
            "WipeLeft" => Ok(Transition::WipeLeft),
            "WipeRight" => Ok(Transition::WipeRight),
            "CircleOpen" => Ok(Transition::CircleOpen),
            "FadeBlack" => Ok(Transition::FadeBlack),
            "FadeWhite" => Ok(Transition::FadeWhite),
            _ => Err(format!("Invalid transition: {}", s)),
        }
    }
}