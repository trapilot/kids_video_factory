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