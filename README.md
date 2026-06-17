# AI Kids Video Factory

An autonomous multi-agent system that generates educational short-form videos for children and publishes them to social media platforms.

## Features

* Multi-agent workflow architecture
* Educational content generation
* AI-generated images
* AI-generated voice narration
* Automatic video rendering
* YouTube Shorts upload
* TikTok upload
* Topic deduplication
* Character-driven storytelling

---

## Architecture

```text
Planner
  ↓
Writer
  ↓
Builder
  ↓
Renderer
  ↓
Publisher
  ↓
End
```

---

## Supported Providers

### LLM

* Gemini
* OpenRouter

### Image Generation

* OpenAI GPT Image
* Gemini Image
* Cloudflare

### Text-to-Speech

* OpenAI TTS
* Gemini TTS
* ElevenLabs

### Video Rendering

* FFmpeg

---

## Project Structure

```text
src/
├── agents/
│   ├── gemini.rs
│   ├── openai.rs
│   └── openrouter.rs
│
├── db.rs
├── enums.rs
├── helper.rs
├── macro_rules.rs
├── main.rs
├── models.rs
├── renderer.rs
├── scheduler.rs
├── uploader.rs
├── workflow.rs
└── main.rs
```

---

## Environment Variables

```env
HF_API_KEY=
OPENAI_API_KEY=
GEMINI_API_KEY=
OPENROUTER_API_KEY=
ELEVENLABS_API_KEY=

CF_ACCOUNT_ID=
CF_API_TOKEN=

YOUTUBE_CLIENT_ID=
YOUTUBE_CLIENT_SECRET=
YOUTUBE_REFRESH_TOKEN=

TIKTOK_ACCESS_TOKEN=
```

---

## Running

```bash
cargo run
```

---

## Example Workflow

1. Planner selects a topic.
2. Writer generates video artifact.
3. Builder generates scene assets.
4. Renderer creates the final video.
5. Publisher publishes the video to Youtube|Tiktok.

---

## Example Output

```text
output/
├── 20200101_001/
│   ├── session_id
|           ├── audios/
│           │      ├── scene_1.mp3
│           │      ├── scene_2.mp3
│           │      └── scene_3.mp3
│           ├── images
│           │      ├── scene_1.png
│           │      ├── scene_2.png
│           │      └── scene_3.png
│           ├── videos
│           │      ├── scene_1.mp4
│           │      ├── scene_2.mp4
│           │      └── scene_3.mp4
|           ├── list.txt
|           └── final_video.mp4
│
```

---

## Roadmap

* [ ] Subtitle generation
* [ ] Background music generation
* [ ] Thumbnail generation
* [ ] Multi-language support
* [ ] Content quality review agent
* [ ] SEO optimization agent
* [ ] Automatic scheduling and publishing

---

## License

MIT License
