# Prompt template (v0.1.7)

> `crates/prompt-template` — `{{var}}` templated system prompts.

## Why

Hard-coded system prompts can't adapt to project metadata (language,
framework, git branch). `prompt-template` adds `{{var}}` substitution
with nested dot-path lookups, so a single template can serve many projects.

## API

```rust
use prompt_template::Template;

let t = Template::new("You are coding in {{project.lang}}.");
let s = t.render(&serde_json::json!({"project": {"lang": "rust"}}))?;
// → "You are coding in rust."
```

Built-in per-role templates:

```rust
use prompt_template::role_template;
let t = role_template("coder")?;
```

| Role | Template |
|------|----------|
| `coder` | senior engineer, project + lang, minimal changes |
| `reviewer` | correctness > security > style, repo name |
| `planner` | break into small steps, goal + constraints |

## Edge cases

- Unknown placeholder → left literal as `{{name}}`
- `Value::Null` lookup → treated as missing
- `Value::Number/Bool` → stringified

## What's NOT in v0.1.7

- Conditional blocks (`{{#if x}}...{{/if}}`)
- Loop constructs (`{{#each items}}`)
- Default values (`{{name\|"anonymous"}}`)
- Template inheritance / partials
