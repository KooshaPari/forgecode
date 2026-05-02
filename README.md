<h1 align="center">⚒️ Forge: AI-Enhanced Terminal Development Environment</h1>

> **Phenotype-org fork of [tailcallhq/forgecode](https://github.com/tailcallhq/forgecode).**
> Phenotype-org additions: `deny.toml` + `cargo-deny.yml` CI bootstrapped 2026-05-01.
> All upstream docs mirror [upstream README](https://github.com/tailcallhq/forgecode/blob/main/README.md).

<p align="center">A comprehensive coding agent that integrates AI capabilities with your development environment</p>

<p align="center"><code>curl -fsSL https://forgecode.dev/cli | sh</code></p>

[![CI Status](https://img.shields.io/github/actions/workflow/status/tailcallhq/forgecode/ci.yml?style=for-the-badge)](https://github.com/tailcallhq/forgecode/actions)
[![GitHub Release](https://img.shields.io/github/v/release/tailcallhq/forgecode?style=for-the-badge)](https://github.com/tailcallhq/forgecode/releases)
[![Discord](https://img.shields.io/discord/1044859667798568962?style=for-the-badge&cacheSeconds=120&logo=discord)](https://discord.gg/kRZBPpkgwq)

---

## Quickstart

To get started with Forge, run the command below:

```bash
curl -fsSL https://forgecode.dev/cli | sh
```

On first run, Forge will guide you through setting up your AI provider credentials using the interactive login flow. Alternatively, you can configure providers beforehand:

```bash
# Configure your provider credentials interactively
forge provider login

# Then start Forge
forge
```
That's it! Forge is now ready to assist you with your development tasks.

## Usage Examples

Forge can be used in different ways depending on your needs. Here are some common usage patterns:

<details>
<summary><strong>Code Understanding</strong></summary>

```
> Can you explain how the authentication system works in this codebase?
```

Forge will analyze your project's structure, identify authentication-related files, and provide a detailed explanation of the authentication flow, including the relationships between different components.

</details>

<details>
<summary><strong>Implementing New Features</strong></summary>

```
> I need to add a dark mode toggle to our React application. How should I approach this?
```

Forge will suggest the best approach based on your current codebase, explain the steps needed, and even scaffold the necessary components and styles for you.

</details>

<details>
<summary><strong>Debugging Assistance</strong></summary>

```
> I'm getting this error: "TypeError: Cannot read property 'map' of undefined". What might be causing it?
```

Forge will analyze the error, suggest potential causes based on your code, and propose different solutions to fix the issue.

</details>

<details>
<summary><strong>Code Reviews</strong></summary>

```
> Please review the code in src/components/UserProfile.js and suggest improvements
```

Forge will analyze the code, identify potential issues, and suggest improvements for readability, performance, security, and maintainability.

</details>

<details>
<summary><strong>Learning New Technologies</strong></summary>

```
> I want to integrate GraphQL into this Express application. Can you explain how to get started?
```

Forge will provide a tailored tutorial on integrating GraphQL with Express, using your specific project structure as context.

</details>

<details>
<summary><strong>Database Schema Design</strong></summary>

```
> I need to design a database schema for a blog with users, posts, comments, and categories
```

Forge will suggest an appropriate schema design, including tables/collections, relationships, indexes, and constraints based on your project's existing database technology.

</details>

<details>
<summary><strong>Refactoring Legacy Code</strong></summary>

```
> Help me refactor this class-based component to use React Hooks
```

Forge can help modernize your codebase by walking you through refactoring steps and implementing them with your approval.

</details>

<details>
<summary><strong>Git Operations</strong></summary>

```
> I need to merge branch 'feature/user-profile' into main but there are conflicts
```

Forge can guide you through resolving git conflicts, explaining the differences and suggesting the best way to reconcile them.

</details>

## Why Forge?

Forge is designed for developers who want to enhance their workflow with AI assistance while maintaining full control over their development environment.

- **Zero configuration** - Just add your API key and you're ready to go
- **Seamless integration** - Works right in your terminal, where you already work
- **Multi-provider support** - Use OpenAI, Anthropic, or other LLM providers
- **Secure by design** - Restricted shell mode limits file system access and prevents unintended changes
- **Open-source** - Transparent, extensible, and community-driven

Forge helps you code faster, solve complex problems, and learn new technologies without leaving your terminal.

---

## Installation

```bash
# YOLO
curl -fsSL https://forgecode.dev/cli | sh

# Package managers
nix run github:tailcallhq/forgecode # for latest dev branch
```

## Community

Join our vibrant Discord community to connect with other Forge users and contributors, get help with your projects, share ideas, and provide feedback!

[![Discord](https://img.shields.io/discord/1044859667798568962?style=for-the-badge&cacheSeconds=120&logo=discord)](https://discord.gg/kRZBPpkgwq)

## Documentation

For comprehensive documentation on all features and capabilities, please visit the [documentation site](https://github.com/tailcallhq/forgecode/tree/main/docs).

---

<!--
╔══════════════════════════════════════════════════════════════════════════════╗
║  UPSTREAM DOCUMENTATION BELOW — mirrors tailcallhq/forgecode main README   ║
║  See: https://github.com/tailcallhq/forgecode/blob/main/README.md           ║
╚══════════════════════════════════════════════════════════════════════════════╝
-->
