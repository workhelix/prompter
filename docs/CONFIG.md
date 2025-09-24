# Configuration Guide

This document provides comprehensive guidance on configuring the `prompter` CLI tool.

## Overview

The `prompter` tool uses TOML configuration files to define profiles and their dependencies. Each profile specifies a collection of markdown files and/or other profiles to be concatenated into a single output.

## Configuration File Locations

### Default Configuration Path
- **Config file**: `$HOME/.config/prompter/config.toml`
- **Library directory**: `$HOME/.local/prompter/library/`

### Custom Configuration Files
You can override the default configuration file using the global `--config` flag:

```bash
# Use a custom config file
prompter --config /path/to/custom/config.toml <profile>
prompter --config ./project/config.toml list
```

When using a custom config file:
- The library directory becomes `{config_directory}/library/`
- For example, if your config is at `/project/config.toml`, the library will be at `/project/library/`

## Configuration File Format

The configuration file uses TOML format with the following structure:

### Basic Profile Definition

```toml
[profile_name]
depends_on = ["file1.md", "file2.md", "other_profile"]
```

### Complete Example

```toml
# Global post-prompt (optional)
post_prompt = "Additional instructions to append to all profiles"

# Basic profile with file dependencies
[python.api]
depends_on = ["api/basics.md", "api/authentication.md"]

# Profile with mixed dependencies (files and other profiles)
[general.testing]
depends_on = ["python.api", "testing/unit.md", "testing/integration.md"]

# Complex profile with nested dependencies
[full.stack]
depends_on = [
  "database/setup.md",
  "python.api",
  "frontend/react.md",
  "general.testing"
]
```

## Configuration Options

### Profile Sections

Each profile is defined as a TOML section with the following format:

```toml
[profile_name]
depends_on = [list_of_dependencies]
```

**Profile Names:**
- Can contain dots (e.g., `python.api`, `general.testing`)
- Are case-sensitive
- Must be unique within the configuration file
- Can reference other profiles for hierarchical dependencies

**Dependencies Array:**
- Must be an array of strings
- Can span multiple lines for readability
- Each dependency can be either:
  - A markdown file path (relative to library directory)
  - Another profile name

### Global Configuration

#### Post-Prompt Text
You can define a global post-prompt that will be appended to all profile outputs:

```toml
post_prompt = "Remember to follow coding best practices"

[profile1]
depends_on = ["file1.md"]
```

**Post-Prompt Priority (highest to lowest):**
1. CLI argument (`--post-prompt` or `-P`)
2. Configuration file `post_prompt` setting
3. Default post-prompt

### Multi-line Arrays

For better readability, dependency arrays can span multiple lines:

```toml
[complex.profile]
depends_on = [
  "docs/introduction.md",
  "docs/setup.md",
  "api/endpoints.md",
  "examples/basic.md",
  "examples/advanced.md"
]
```

## File Dependencies

### File Path Resolution
- All file paths are relative to the library directory
- Paths use forward slashes (`/`) on all platforms
- Only `.md` files are treated as file dependencies (case-insensitive)
- Non-`.md` dependencies are treated as profile references

### File Organization
```
$HOME/.local/prompter/library/
├── api/
│   ├── basics.md
│   └── authentication.md
├── testing/
│   ├── unit.md
│   └── integration.md
└── database/
    └── setup.md
```

## Profile Dependencies

### Hierarchical Profiles
Profiles can reference other profiles, creating dependency hierarchies:

```toml
[base]
depends_on = ["common/headers.md", "common/footer.md"]

[python]
depends_on = ["base", "python/syntax.md"]

[python.web]
depends_on = ["python", "web/flask.md", "web/django.md"]
```

### Dependency Resolution
- Dependencies are resolved recursively using depth-first traversal
- Files are deduplicated (first occurrence wins)
- Circular dependencies are detected and cause validation errors
- Order is preserved based on the `depends_on` sequence

## Command-Line Options

### Profile Rendering Options

#### Separator
Control what appears between concatenated files:

```bash
# No separator (default)
prompter profile_name

# Custom separator
prompter --separator "\n---\n" profile_name
prompter -s "\n---\n" profile_name
```

#### Pre-prompt Override
Override the default pre-prompt text:

```bash
# Custom pre-prompt
prompter --pre-prompt "Custom instructions" profile_name
prompter -p "Custom instructions" profile_name
```

#### Post-prompt Override
Override the default/configured post-prompt text:

```bash
# Custom post-prompt
prompter --post-prompt "Final instructions" profile_name
prompter -P "Final instructions" profile_name
```

### Escape Sequences
Command-line arguments support escape sequences:
- `\n` → newline
- `\t` → tab
- `\r` → carriage return
- `\"` → quote
- `\\` → backslash

Example:
```bash
prompter --separator "\n---\n" profile_name
```

## Output Structure

All rendered profiles follow this structure:

```
[Pre-prompt text]

[System info: "Today is YYYY-MM-DD, and you are running on a ARCH/OS system."]

[File 1 content]
[Optional separator]
[File 2 content]
[Optional separator]
...

[Post-prompt text]
```

### Default Content

**Pre-prompt (default):**
```
You are an LLM coding agent. Here are invariants that you must adhere to. Please respond with 'Got it' when you have studied these and understand them. At that point, the operator will give you further instructions. You are *not* to do anything to the contents of this directory until you have been explicitly asked to, by the operator.
```

**Post-prompt (default):**
```
Now, read the @AGENTS.md and @CLAUDE.md files in this directory, if they exist.
```

## Configuration Management Commands

### Initialization
Create default configuration and library structure:

```bash
prompter init
```

This creates:
- `$HOME/.config/prompter/config.toml` with sample profiles
- `$HOME/.local/prompter/library/` with example markdown files
- Only creates files that don't already exist (non-destructive)

### Validation
Validate configuration for errors:

```bash
# Validate default config
prompter validate

# Validate custom config
prompter --config ./custom.toml validate
```

Validation checks:
- All referenced profiles exist
- All referenced markdown files exist
- No circular dependencies
- TOML syntax is valid

### Listing Profiles
List all available profiles:

```bash
# List from default config
prompter list

# List from custom config
prompter --config ./custom.toml list
```

## Error Handling

### Common Configuration Errors

**Missing Profile:**
```
Unknown profile: nonexistent (referenced by [myprofile])
```

**Missing File:**
```
Missing file: /path/to/library/missing.md (referenced by [myprofile])
```

**Circular Dependency:**
```
Cycle detected: profile1 -> profile2 -> profile1
```

**Invalid TOML Syntax:**
```
Invalid depends_on array for [profile]: Unterminated string in array
```

### Troubleshooting Steps

1. **Validate your configuration:**
   ```bash
   prompter validate
   ```

2. **Check file paths are relative to library directory**

3. **Ensure all referenced profiles exist**

4. **Verify TOML syntax with arrays properly formatted**

5. **Use absolute paths for custom config files**

## Advanced Usage

### Development Workflows
Structure profiles for different development contexts:

```toml
# Base development context
[dev.base]
depends_on = ["dev/setup.md", "dev/standards.md"]

# API development
[dev.api]
depends_on = ["dev.base", "api/design.md", "api/testing.md"]

# Frontend development
[dev.frontend]
depends_on = ["dev.base", "frontend/components.md", "frontend/testing.md"]

# Full-stack development
[dev.fullstack]
depends_on = ["dev.api", "dev.frontend", "deployment/docker.md"]
```

### Project-Specific Configurations
Use custom configurations for different projects:

```bash
# Project A
prompter --config ./projects/a/config.toml dev.api

# Project B
prompter --config ./projects/b/config.toml dev.frontend
```

### Template and Snippet Management
Organize reusable content:

```toml
[templates.basic]
depends_on = ["templates/header.md", "templates/footer.md"]

[snippets.python]
depends_on = ["snippets/imports.md", "snippets/logging.md"]

[project.new]
depends_on = ["templates.basic", "snippets.python", "docs/readme-template.md"]
```

## Integration with Other Tools

The `prompter` tool is designed to work well in development pipelines:

```bash
# Pipe to LLM tools
prompter python.api | llm

# Save to file for later use
prompter --separator "\n---\n" full.stack > context.md

# Use in scripts
#!/bin/bash
CONTEXT=$(prompter dev.base)
echo "$CONTEXT" | your-ai-tool
```