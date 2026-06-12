# Create3D dependency rules

These rules mirror the master architecture document and apply from Month 1 onward.

## Layer boundaries

| Layer | May depend on | Must not depend on |
|---|---|---|
| `c3d-core` | std, math, tracing | scene, renderer, editor, AI, robotics |
| `scene/*` | `c3d-core`, schema, asset IDs | editor UI, renderer backend |
| `renderer/*` | scene query types, geometry caches | editor UI |
| `editor/*` | engine, scene, renderer, asset | cloud-only services as hard requirement |
| `ai/*` | scene ops, tool protocol | renderer backend internals |
| `robotics/*` | scene, transform, asset, sidecar protocol | editor UI |
| `plugins/*` | SDK/API crates | unstable internal modules unless built-in |
| `cloud/*` | sync/asset/server APIs | desktop UI |

## Public vs internal APIs

Stable:

- scene operation schema,
- component schema format,
- asset manifest format,
- plugin manifest format,
- AI tool protocol,
- robotics bridge protocol.

Unstable:

- renderer backend internals,
- ECS backend details,
- cache layouts,
- importer internals,
- UI implementation details.

## Testing expectations

- Scene replay tests must not require ECS or renderer state.
- Golden scenes live under `Create3D/tests/golden-scenes/fixtures/`.
- Importers and scene ops should gain fuzz tests as they land.
