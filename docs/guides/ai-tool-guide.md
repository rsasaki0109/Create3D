# AI tool guide (Alpha)

Create3D Copilot and future agents mutate scenes only through typed tools with permission checks, preview, and approval.

## Architecture

```text
User prompt
  -> CopilotEngine (c3d-ai-copilot)
  -> Tool registry / validator (c3d-ai-tool-protocol)
  -> Proposed SceneOperation bundle
  -> Preview in viewport
  -> Approve / Reject
  -> Transaction commit
```

## Built-in tools (mock provider)

The mock Copilot provider supports read-only scene questions and a small write set:

- Translate selected entity
- Rename selected entity
- Create a named entity

Write tools require scene-write permission and produce a preview before commit.

## Adding a tool

1. Define the tool schema in `c3d-ai-tool-protocol`.
2. Register it in the builtin registry with required permissions.
3. Map validated arguments to `SceneOperation` values in the copilot executor.
4. Add tests in `c3d-ai-tool-protocol` and `c3d-ai-copilot`.

## Safety rules

- No direct SceneDB mutation from model output.
- Validate all arguments before preview.
- Record provenance on committed transactions when available.
- Block destructive or unsupported ops in collaboration sync (see `c3d-sync` policy).

## Branch proposals

Approved Copilot bundles can be shared as branch proposals from the Collaboration panel. This is a review artifact, not automatic merge.

## Local/offline mode

Alpha uses the mock local provider only. Remote model providers should follow the same tool-only path when added.
