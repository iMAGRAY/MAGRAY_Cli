# MAGRAY CLI Product Overview

MAGRAY CLI is an intelligent command-line interface agent that combines beautiful animations with LLM integration. It's designed as a minimalist yet effective AI-powered CLI tool.

## Core Purpose
- **Interactive AI Chat**: Natural language conversations with various LLM providers
- **Tool Integration**: Smart routing to file operations, git commands, web search, and shell execution
- **Beautiful UX**: Professional ASCII art, animated spinners, and colored interface with typing effects

## Key Features
- Multi-provider LLM support (OpenAI, Anthropic, local models)
- Unified AI agent that intelligently decides between chat and tool usage
- Animated interface with loading spinners and typing effects
- Cross-platform shell command execution
- File operations with syntax highlighting
- Git integration
- Web search capabilities

## Design Philosophy
- **Minimalism over complexity**: Simple architecture, fast build times (~30 seconds)
- **Effect over features**: Professional animations and UX over extensive functionality
- **Performance first**: ~2MB executable, minimal dependencies
- **Natural interaction**: Users don't need to think about specific commands

## What's NOT included (by design)
- Conversation memory (each request is independent)
- Task scheduling
- Vector search
- File operations beyond basic read/write