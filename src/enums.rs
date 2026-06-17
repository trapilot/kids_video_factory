
use serde::{Deserialize, Serialize};

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


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentNode {
    Planner,
    Writer,
    Builder,
    Renderer,
    Publisher,
    End,
}
impl AgentNode {
    pub fn back(&self) -> Option<Self> {
        use AgentNode::*;

        match self {
            Planner => None,
            Writer => Some(Planner),
            Builder => Some(Writer),
            Renderer => Some(Builder),
            Publisher => Some(Renderer),
            End => Some(Publisher),
        }
    }
}
impl Default for AgentNode {
    fn default() -> Self {
        AgentNode::Planner
    }
}
impl std::fmt::Display for AgentNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AgentNode::Planner => "planner",
            AgentNode::Writer => "writer",
            AgentNode::Builder => "builder",
            AgentNode::Renderer => "renderer",
            AgentNode::Publisher => "publisher",
            AgentNode::End => "end",
        };

        write!(f, "{}", s)
    }
}
