use serde::{Deserialize, Serialize};
use chrono::{Datelike, Utc};

use crate::enums::*;


pub struct Character {
    pub enabled: bool,
    pub spotlight: bool,

    pub fullname: &'static str,
    pub nickname: &'static str,
    pub voice_id: Option<&'static str>,
    pub birth_year: u16,

    pub role: &'static str,
    pub family_role: &'static str,
    pub story_function: &'static str,

    pub profession: &'static str,
    pub topic_domain: &'static str,

    pub catchphrase: &'static str,
    pub speaking_style: &'static str,
    pub visual_template: &'static str,

    pub strengths: &'static str,
    pub weaknesses: &'static str,
    pub core_values: &'static str,

    pub relationship_prompt: &'static str,
    pub personality_prompt: &'static str,
}

impl Character {
    pub fn age(&self) -> i32 {
        let current_year = Utc::now().year();
        current_year - self.birth_year as i32
    }

    pub fn name(&self, language: &str) -> &str {
        if language == "English" {
            // return self.nickname;
        }
        self.fullname
    }
    
    pub fn visual_anchor(&self) -> String {
        self.visual_template.replace("{age}", &self.age().to_string())
    }

    pub fn find_char(name: &str) -> Option<&'static Character> {
        EDU_CHARACTERS
            .iter()
            .find(|c| c.fullname == name || c.nickname == name)
    }

    pub fn find_chars(characters: &Vec<String>) -> Vec<&'static Character> {
        let character_set: std::collections::HashSet<&str> = characters
            .iter()
            .map(|s| s.as_str())
            .collect();

        EDU_CHARACTERS
            .iter()
            .filter(|c| {
                return character_set.contains(c.fullname) || character_set.contains(c.nickname)
            })
            .collect()
    }

    pub fn main_char() -> &'static Character {
        EDU_CHARACTERS.first().unwrap()
    }

    pub fn spotlight_chars() -> Vec<&'static Character> {
        return EDU_CHARACTERS
            .iter()
            .filter(|c| c.enabled && c.spotlight)
            .collect();
    }

    pub fn relation_chars() -> Vec<&'static Character> {
        return EDU_CHARACTERS
            .iter()
            .filter(|c| c.enabled && !c.spotlight)
            .collect();
    }
}

pub const EDU_CHARACTERS: &[Character] = &[
    Character {
        enabled: true,
        spotlight: true,

        fullname: "Quốc Khánh",
        nickname: "Bin",
        voice_id: None,
        birth_year: 2020,

        role: "Protagonist",
        family_role: "child",
        story_function: "Main protagonist who initiates most adventures and learning experiences.",

        profession: "Student",
        topic_domain: "Curiosity, Exploration, Everyday Learning",

        catchphrase: "I want to try it!",
        speaking_style: "Very energetic, speaks quickly, asks many 'why' questions, expresses excitement openly.",
        visual_template: "a 3D Pixar style illustration of a cute Vietnamese boy age {age}, short black hair, bright curious eyes, yellow t-shirt, blue shorts, energetic pose, white background",
        // visual_template: "A 3D Pixar style illustration of a cute Vietnamese boy, age {age}, short black hair, bright curious eyes, wearing a black sports cap with white trim on the brim and the word 'Hello!' written in white text with a small smile icon underneath. He is wearing a black sleeveless athletic tank top with white stripes on the shoulders, matching black sports shorts with white stripes down the sides and a prominent white Adidas logo on the left leg. On his feet, he wears a pair of red and grey athletic slide sandals with three bold white stripes across the wide red straps. He is standing in a confident pose with his arms crossed over his chest, isolated on a clean white background",

        strengths: "Curious, brave, friendly, imaginative, willing to help others.",
        weaknesses: "Impulsive, impatient, easily distracted, sometimes acts before thinking.",
        core_values: "Curiosity, friendship, courage, kindness.",

        relationship_prompt: "Bin is the son of Nam and Diem Suong. He is the best friend of Hoang Yen. Hoang Yen often helps and protects him when he gets into trouble.",
        personality_prompt: "A lively and energetic Vietnamese boy who loves exploring everything around him and learning through adventure.",
    },

    Character {
        enabled: true,
        spotlight: false,

        fullname: "Hoàng Yến",
        nickname: "Hoàng Yến",
        voice_id: None,
        birth_year: 2020,

        role: "Best Friend",
        family_role: "friend",
        story_function: "Voice of reason and emotional support for Bin.",

        profession: "Student",
        topic_domain: "Learning, Friendship, Kindness",

        catchphrase: "Để mình giúp nhé!",
        speaking_style: "Gentle, polite, calm, encouraging and supportive.",
        visual_template: "a 3D Pixar style illustration of a cute Vietnamese girl age {age}, long black hair with pink ribbons, pink dress, holding books, warm smile, white background",
        
        strengths: "Intelligent, hardworking, loyal, responsible, brave when protecting friends.",
        weaknesses: "Can worry too much, dislikes conflict, sometimes overthinks.",
        core_values: "Kindness, friendship, honesty, responsibility.",
        
        relationship_prompt: "Hoang Yen is Bin's classmate and best friend. She often helps him solve problems and stands up for him when needed.",
        personality_prompt: "A sweet and intelligent girl who loves learning and always supports her friends.",
    },

    Character {
        enabled: true,
        spotlight: false,

        fullname: "Nam",
        nickname: "Nam",
        voice_id: None,
        birth_year: 1990,

        role: "Father",
        family_role: "father",
        story_function: "Mentor figure who explains concepts and guides Bin's learning.",

        profession: "Software Engineer",
        topic_domain: "Technology, Logic, Problem Solving",

        catchphrase: "What do you think about this?",
        speaking_style: "Calm, logical, patient, explains things step by step.",
        visual_template: "a 3D Pixar style illustration of a friendly Vietnamese father in his mid 30s, glasses, polo shirt, holding a laptop, white background",

        strengths: "Wise, patient, analytical, dependable.",
        weaknesses: "Perfectionist, strict with rules, sometimes too serious.",
        core_values: "Responsibility, honesty, critical thinking, discipline.",

        relationship_prompt: "Nam is Bin's father and Diem Suong's husband. Bin often asks him questions about the world.",
        personality_prompt: "A thoughtful software engineer who enjoys teaching children how things work.",
    },

    Character {
        enabled: true,
        spotlight: false,

        fullname: "Diễm Sương",
        nickname: "Suong",
        voice_id: None,
        birth_year: 1993,

        role: "Mother",
        family_role: "mother",
        story_function: "Emotional anchor of the family and teacher of life lessons.",

        profession: "Elementary School Teacher",
        topic_domain: "Education, Family, Life Skills",

        catchphrase: "Listen to me!",
        speaking_style: "Warm, caring, nurturing, occasionally firm when teaching lessons.",
        visual_template: "a 3D Pixar style illustration of a gentle Vietnamese mother in her early 30s, long black hair, pastel blouse, holding a book and cooking spoon, warm smile, white background",

        strengths: "Patient, loving, organized, empathetic.",
        weaknesses: "Can become worried easily, sometimes overprotective.",
        core_values: "Family, respect, kindness, good habits.",
        
        relationship_prompt: "Diem Suong is Bin's mother and Nam's wife. She helps Bin learn responsibility and good behavior.",
        personality_prompt: "A caring elementary school teacher who loves her family and wants children to grow into kind people.",
    },
];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoMetadata {
    pub title: String,
    pub video_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Timeline {
    pub title: String,
    pub clips: Vec<Clip>,
    pub render_mode: RenderMode,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Storyboard {
    pub title: String,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct StoryContext {
    pub topic: String,
    pub main_character: String,
    pub spotlight_characters: Vec<String>,
    pub supporting_characters: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Clip {
    pub scene_id: u8,
    pub audio_path: String,
    pub visual_path: String,
    pub start_time: f64,
    pub end_time: f64,
    pub duration: f64,
    pub acrossfade: f64,
    pub transition: Transition,
    pub motion: Motion,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scene {
    pub scene_id: u8,
    pub duration: u8,
    pub motion: Motion,
    pub transition: Transition,
    pub visual_prompt: String,
    pub voice_segments: Vec<VoiceSegment>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VoiceSegment {
    pub text: String,
    pub speaker: String,
    pub voice_id: Option<String>,
}