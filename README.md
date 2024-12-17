Apprentice is an assistant that helps to manage cloud resources using natural language.
It translates natural language descritpion into a cloud CLI tool command and executes it.

### API providers

- Anthropic (Claude models)
- OpeanAI (GPT models)
- Google Cloud Platform (Gemini)

### Installation

For Rust users:

```bash
cargo install --git https://github.com/ai-gentic/apprentice
```

Download [binaries](https://github.com/ai-gentic/apprentice/releases).

### Usage

![apprentice --goal=gcp --model=gemini-1.5-pro-002 --model-provider=gcp --api-key=<your-key> --message="List all cloud sql instances"](doc/img.gif)
