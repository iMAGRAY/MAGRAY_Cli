# MAGRAY Policy System

This document describes the JSON-based policy system used to allow/deny tools and commands at runtime.

Format
- Document: `{ "rules": [ PolicyRule, ... ] }`
- PolicyRule:
  - `subject_kind`: `"Tool" | "Command"`
  - `subject_name`: string, exact match or `"*"` for wildcard
  - `when_contains_args`: optional object with key/value pairs that must be present in the evaluated args
  - `action`: `"Allow" | "Deny"`
  - `reason`: optional human-readable reason

Evaluation
- Rules are evaluated in order, last matching rule takes precedence (override semantics)
- Default decision when no rule matches: Allow

Loading & Precedence
- Effective policy is computed as the merge (append) of these documents in order (last wins):
  1. Built-in default document (secure-by-default: denies `shell_exec`)
  2. File from `${MAGRAY_HOME}/policy.json` (by default `~/.magray/policy.json`)
  3. File from `MAGRAY_POLICY_PATH` env var (if set)
  4. Inline JSON from `MAGRAY_POLICY_JSON` env var (if set)

Common Use-Cases
- Deny dangerous tools globally:
```json
{
  "rules": [
    { "subject_kind": "Tool", "subject_name": "shell_exec", "action": "Deny", "reason": "no shell" }
  ]
}
```

- Deny web_fetch for specific domain:
```json
{
  "rules": [
    {
      "subject_kind": "Tool",
      "subject_name": "web_fetch",
      "when_contains_args": { "domain": "example.com" },
      "action": "Deny",
      "reason": "block example.com"
    }
  ]
}
```

- Deny memory backup/restore commands:
```json
{
  "rules": [
    { "subject_kind": "Command", "subject_name": "memory.backup", "action": "Deny" },
    { "subject_kind": "Command", "subject_name": "memory.restore", "action": "Deny" }
  ]
}
```

- Allow overriding default deny via env JSON (highest precedence):
Set `MAGRAY_POLICY_JSON` to:
```json
{ "rules": [ { "subject_kind": "Tool", "subject_name": "shell_exec", "action": "Allow" } ] }
```

Notes
- `web_fetch` policy uses `domain` arg which is extracted from the `url` during policy evaluation.
- `web_search` policy can use a `keyword` arg extracted from the natural-language query (e.g. `internal`, `secret`).