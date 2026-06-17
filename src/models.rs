use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::helper::*;

#[derive(Debug, Clone)]
pub struct Character {
    pub enabled: bool,
    pub spotlight: bool,

    pub name: &'static str,
    pub nickname: &'static str,
    pub voice_id: &'static str,
    pub birth_year: i32,

    pub role: &'static str,
    pub family_role: &'static str,
    pub story_function: &'static str,

    pub profession: &'static str,
    pub topic_domain: &'static str,

    pub catchphrase: &'static str,
    pub speaking_style: &'static str,
    pub visual_anchor: &'static str,

    pub strengths: &'static str,
    pub weaknesses: &'static str,
    pub core_values: &'static str,

    pub relationship_prompt: &'static str,
    pub personality_prompt: &'static str,
}

// w2KTJ6MO4SIK6nWK4YH8 pNInz6obpgDQGcFmaJgB
pub const BASE_VOICE: &str = "pNInz6obpgDQGcFmaJgB";
pub const EDU_CHARACTERS: &[Character] = &[
    Character {
        enabled: true,
        spotlight: true,

        name: "Quốc Khánh",
        nickname: "Bin",
        voice_id: BASE_VOICE, // Adam
        birth_year: 2020,

        role: "Protagonist",
        family_role: "child",
        story_function: "Main protagonist who initiates most adventures and learning experiences.",

        profession: "Student",
        topic_domain: "Curiosity, Exploration, Everyday Learning",

        catchphrase: "I want to try it!",
        speaking_style: "Very energetic, speaks quickly, asks many 'why' questions, expresses excitement openly.",
        visual_anchor: "a 3D Pixar style illustration of a cute Vietnamese boy age 6, short black hair, bright curious eyes, yellow t-shirt, blue shorts, energetic pose, white background",

        strengths: "Curious, brave, friendly, imaginative, willing to help others.",
        weaknesses: "Impulsive, impatient, easily distracted, sometimes acts before thinking.",
        core_values: "Curiosity, friendship, courage, kindness.",

        relationship_prompt: "Bin is the son of Nam and Diem Suong. He is the best friend of Hoang Yen. Hoang Yen often helps and protects him when he gets into trouble.",
        personality_prompt: "A lively and energetic Vietnamese boy who loves exploring everything around him and learning through adventure.",
    },

    Character {
        enabled: true,
        spotlight: false,

        name: "Hoàng Yến",
        nickname: "Yến",
        voice_id: BASE_VOICE, // Bella
        birth_year: 2020,

        role: "Best Friend",
        family_role: "friend",
        story_function: "Voice of reason and emotional support for Bin.",

        profession: "Student",
        topic_domain: "Learning, Friendship, Kindness",

        catchphrase: "Để mình giúp nhé!",
        speaking_style: "Gentle, polite, calm, encouraging and supportive.",
        visual_anchor: "a 3D Pixar style illustration of a cute Vietnamese girl age 6, long black hair with pink ribbons, pink dress, holding books, warm smile, white background",
        
        strengths: "Intelligent, hardworking, loyal, responsible, brave when protecting friends.",
        weaknesses: "Can worry too much, dislikes conflict, sometimes overthinks.",
        core_values: "Kindness, friendship, honesty, responsibility.",
        
        relationship_prompt: "Hoang Yen is Bin's classmate and best friend. She often helps him solve problems and stands up for him when needed.",
        personality_prompt: "A sweet and intelligent girl who loves learning and always supports her friends.",
    },

    Character {
        enabled: true,
        spotlight: false,

        name: "Nam",
        nickname: "Ba Nam",
        voice_id: BASE_VOICE,
        birth_year: 1990,

        role: "Father",
        family_role: "father",
        story_function: "Mentor figure who explains concepts and guides Bin's learning.",

        profession: "Software Engineer",
        topic_domain: "Technology, Logic, Problem Solving",

        catchphrase: "What do you think about this?",
        speaking_style: "Calm, logical, patient, explains things step by step.",
        visual_anchor: "a 3D Pixar style illustration of a friendly Vietnamese father in his mid 30s, glasses, polo shirt, holding a laptop, white background",

        strengths: "Wise, patient, analytical, dependable.",
        weaknesses: "Perfectionist, strict with rules, sometimes too serious.",
        core_values: "Responsibility, honesty, critical thinking, discipline.",

        relationship_prompt: "Nam is Bin's father and Diem Suong's husband. Bin often asks him questions about the world.",
        personality_prompt: "A thoughtful software engineer who enjoys teaching children how things work.",
    },

    Character {
        enabled: true,
        spotlight: false,

        name: "Diễm Sương",
        nickname: "Mẹ Sương",
        voice_id: BASE_VOICE,
        birth_year: 1993,

        role: "Mother",
        family_role: "mother",
        story_function: "Emotional anchor of the family and teacher of life lessons.",

        profession: "Elementary School Teacher",
        topic_domain: "Education, Family, Life Skills",

        catchphrase: "Listen to me!",
        speaking_style: "Warm, caring, nurturing, occasionally firm when teaching lessons.",
        visual_anchor: "a 3D Pixar style illustration of a gentle Vietnamese mother in her early 30s, long black hair, pastel blouse, holding a book and cooking spoon, warm smile, white background",

        strengths: "Patient, loving, organized, empathetic.",
        weaknesses: "Can become worried easily, sometimes overprotective.",
        core_values: "Family, respect, kindness, good habits.",
        
        relationship_prompt: "Diem Suong is Bin's mother and Nam's wife. She helps Bin learn responsibility and good behavior.",
        personality_prompt: "A caring elementary school teacher who loves her family and wants children to grow into kind people.",
    },
];

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct WorkflowMeta {
    pub status: String, // running | cooldown | failed | done | cancelled
    pub retry_count: i32,
    pub max_retry: i32,
    pub backoff_ms: i32,
    pub last_error: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct VideoState {
    pub target_age: u8,
    pub target_path: String,
    pub target_topic: String,
    pub main_character: String,
    pub spotlight_characters: Vec<String>,
    pub supporting_characters: Vec<String>,
    pub concept_ideas: Vec<String>,
    pub draft_script: String,
    pub final_json: Option<VideoArtifact>,
    pub scene_assets: Vec<SceneAsset>,
    pub current_node: AgentNode,
    pub session_id: String,
    pub video_path: String,
    pub meta: WorkflowMeta,

    // 👇 add tracking flags
    pub youtube_uploaded: bool,
    pub tiktok_uploaded: bool,
}
impl VideoState {
    pub fn new() -> Self {
        let target_char = EDU_CHARACTERS.first().unwrap();
        let target_age = now_age(target_char.birth_year) as u8;
        let session_id = Uuid::new_v4().to_string();
        let target_path = format!(
            "./output/{}_{}/{}",
            now_ymd(),
            left_pad(target_age as u32, 3),
            session_id,
        );
        
        Self {
            session_id,
            target_age,
            target_path,
            target_topic: "".into(),
            current_node: AgentNode::Planner,
            main_character: "".into(),
            spotlight_characters: vec![],
            supporting_characters: vec![],
            concept_ideas: vec![],
            draft_script: "".into(),
            scene_assets: vec![],
            final_json: None,
            video_path: "".into(),
            youtube_uploaded: false,
            tiktok_uploaded: false,
            meta: WorkflowMeta {
                status: "running".into(),
                retry_count: 0,
                max_retry: 3,
                backoff_ms: 1000,
                last_error: None,
                updated_at: chrono::Local::now().to_string(),
            },
        }
    }

    pub fn is_finished(&self) -> bool {
        return self.meta.status == "done" || self.meta.status == "cancelled"
    }

    pub fn done(&mut self) -> &Self {
        self.meta.updated_at = now_rfc();
        self.meta.last_error = None;
        self.meta.status = "done".into();
        self
    }

    pub fn revert(&mut self) -> &Self {
        if let Some(prev) = self.current_node.back() {
            self.current_node = prev;
        }
        self
    }

    pub fn retry(&mut self, e: String) -> &Self {
        self.meta.updated_at = now_rfc();
        self.meta.last_error = Some(e.clone());
        self.meta.retry_count = self.meta.retry_count + 1;
        self.meta.status = "cooldown".into();
        self
    }

    pub fn revived(&mut self) -> &Self {
        self.meta.updated_at = now_rfc();
        self.meta.retry_count = self.meta.max_retry - 1;
        self.meta.status = "revived".into();
        self
    }

    pub fn cancelled(&mut self) -> &Self {
        self.meta.updated_at = now_rfc();
        self.meta.status = "cancelled".into();
        self
    }
    
    pub fn failed(&mut self, e: String) -> &Self {
        self.meta.updated_at = now_rfc();
        self.meta.last_error = Some(e.clone());
        self.meta.status = "failed".into();
        self
    }

    // pub fn reset_meta(&mut self) {
    //     self.meta.status = "running".into();
    //     self.meta.retry_count = 0;
    //     self.meta.backoff_ms = 1000;
    //     self.meta.last_error = None;
    //     self.meta.updated_at = now_rfc();
    // }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoArtifact {
    pub title: String,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceSegment {
    pub speaker: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneAsset {
    pub scene_id: u8,
    pub audio_path: String,
    pub visual_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scene {
    pub scene_id: u8,
    pub duration: u8,
    pub visual_prompt: String,
    pub voice_segments: Vec<VoiceSegment>,
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
