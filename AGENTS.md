# AGENT.md

## Overview

This project uses a multi-agent architecture to automatically generate educational short-form videos for children.

Each agent has a single responsibility and communicates through a shared workflow state.

---

## Workflow

Planner → Writer → Builder → Renderer → Uploader

---

## Agents

### Planner Agent

**Purpose**

Select a character and generate a unique educational topic.

**Input**

* Target age
* Previous video history

**Output**

```json
{
  "topic": "Why do stars twinkle?",
  "main_character": "Leo",
  "spotlight_characters": ["Leo", "Binh", "Minh"],
  "supporting_characters": ["Ba", "Hai"]
}
```

---

### Writer Agent

**Purpose**

Generate a short-form script suitable for YouTube Shorts and TikTok.

**Input**

* Topic
* Character personality

**Output**

```json
{
  "title": "Why Stars Twinkle",
  "scenes": [
    {
      "scene_id": 1,
      "duration": 5,
      "transition": "...",
      "motion": "...",
      "visual_prompt": "...",
      "voice_segments": [
        {
          "speaker": "...",
          "text": "..."
        }
      ]
    }
  ]
}
```

---

### Builder Agent

**Purpose**

Generate images for each scene.

**Provider**

* OpenAI GPT Image
* Gemini Image
* ElevenLabs
* Cloudflare

**Output**

```text
output/{YMY_Index}/{sesstion_id}/images/scene_1.png
output/{YMY_Index}/{sesstion_id}/images/scene_2.png
output/{YMY_Index}/{sesstion_id}/audios/scene_1.mp3
output/{YMY_Index}/{sesstion_id}/audios/scene_2.mp3
...
```

---

### Renderer Agent

**Purpose**

Combine images and audio into scene videos and merge them into the final video.

**Technology**

* FFmpeg

**Output**

```text
output/{YMY_Index}/{sesstion_id}/videos/scene_1.mp4
output/{YMY_Index}/{sesstion_id}/videos/scene_2.mp4
output/{YMY_Index}/{sesstion_id}/final_video.mp4
```

---

### Uploader Agent

**Purpose**

Publish completed videos to social platforms.

**Targets**

* YouTube Shorts
* TikTok

---

## Shared State

Example:

```rust
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
```

---

## Design Principles

1. One responsibility per agent.
2. Agents should be replaceable.
3. Providers are abstracted from workflow logic.
4. Rendering and uploading should remain independent of content generation.
5. Failures should return Result<T, String> instead of panicking.

---

## Future Agents

* SubtitleAgent
* MusicAgent
* ThumbnailAgent
* SEOAgent
* TranslationAgent
* QAAgent
