Apprentice is an assistant that helps to manage cloud resources using natural language.
It translates natural language descritpion using LLM into cloud CLI tool command and executes it.

### API providers

- Anthropic (Claude models)
- OpeanAI (GPT models)
- Google Cloud Platform (Gemini)

### Install

```bash
cargo install --git https://github.com/ai-gentic/apprentice
```

### Usage

![apprentice --goal=gcp --model=gemini-1.5-pro-002 --model-provider=gcp --api-key=<your-key> --message="List all cloud sql instances"](doc/img.gif)
