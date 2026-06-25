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
│   ├── builder.rs
│   ├── cleaner.rs
│   ├── manager.rs
│   ├── planner.rs
│   ├── publisher.rs
│   ├── renderer.rs
│   └── writer.rs
├── entities/
│   ├── character.rs
│   ├── movie.rs
│   └── writer.rs
├── models/
│   ├── auth.rs
│   └── job.rs
├── providers/
│   ├── cf_worker.rs
│   ├── eleven_labs.rs
│   └── gemini.rs
├── uploaders/
│   ├── tiktok.rs
│   └── youtube.rs
├── agent.rs
├── config.rs
├── db.rs
├── main.rs
├── oauth.rs
├── provider.rs
├── uploader.rs
└── workflow.rs
```

---

## Environment Variables

```env
GEMINI_KEY_1=
GEMINI_KEY_2=

ELEVEN_LABS_KEY_1=
ELEVEN_LABS_KEY_2=

CF_WORKER_KEY_1=
CF_WORKER_ACCOUNT_1=
CF_WORKER_KEY_2=
CF_WORKER_ACCOUNT_2=



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
|           ├── shot_1/
│           │      ├── image.png
│           │      ├── audio.mp3
│           │      └── video.mp4
│           ├── shot_2
│           │      ├── image.png
│           │      ├── audio.mp3
│           │      └── video.mp4
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
