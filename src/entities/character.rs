
use chrono::{Datelike, Utc};

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
        visual_template: "A 3D Pixar style illustration of a cute Vietnamese boy, age {age}, short black hair, bright curious eyes, wearing a black sports cap with white trim on the brim and the word 'Hello!' written in white text with a small smile icon underneath. He is wearing a black sleeveless athletic tank top with white stripes on the shoulders, matching black sports shorts with white stripes down the sides and a pair of red and grey athletic slide sandals with three bold white stripes across the red straps. All features, eyeglasses, and clothing details are clear and detailed",

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
        visual_template: "A 3D Pixar-style character illustration of the Vietnamese father, with his precise facial features, welcoming smile, and thin metal-framed spectacles preserved. He is {age} years old and is wearing a well-fitted black crew-neck t-shirt and classic blue denim jeans, with simple dark sneakers. His pose is friendly and forward-facing, set against a clean, subtly graded light grey studio background. The character model is entire and well-lit with soft, warm studio lighting, perfect for a high-quality character render. All features, eyeglasses, and clothing details are clear and detailed",

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
        visual_template: "A 3D Pixar-style character illustration of the Vietnamese mother, with her precise facial features and warm smile preserved. She is {age} years old, wearing her original white sleeveless top and wide-leg white pants, with a pair of metal-framed sunglasses tucked into the neckline. Her pose is friendly and forward-facing, set against a clean, subtly graded light grey studio background. The character model is entire and well-lit with soft, warm studio lighting, perfect for a high-quality character render. All features and clothing details are clear and detailed",

        strengths: "Patient, loving, organized, empathetic.",
        weaknesses: "Can become worried easily, sometimes overprotective.",
        core_values: "Family, respect, kindness, good habits.",
        
        relationship_prompt: "Diem Suong is Bin's mother and Nam's wife. She helps Bin learn responsibility and good behavior.",
        personality_prompt: "A caring elementary school teacher who loves her family and wants children to grow into kind people.",
    },
];