# Known limitations (Beta)

Create3D **0.1.1-beta** is a public prototype. Expect rough edges.

## Editor

- Desktop persists the demo project under the system temp directory by default.
- No multi-document UI; one project at a time.
- Undo/redo covers scene transactions but not asset blob history.
- UI polish is functional, not production-grade.

## Rendering

- wgpu backend only; advanced native backends are not shipped.
- Large mesh scenes are not fully optimized; point clouds and splats use chunk residency but not full streaming LOD.

## Import

- glTF: supported paths only; exotic extensions may fail with path-qualified errors.
- PLY / 3DGS: ASCII-oriented importers; binary PLY support is limited.
- URDF: preview arm and common links; mesh packages with external assets may need manual path fixes.

## Export

- GLB export writes mesh hierarchy snapshots with base-color factors and embedded base-color textures; animations and point clouds are not exported yet.
- USDA export writes mesh hierarchy snapshots with base-color `displayColor`; materials, animations, and point clouds are not exported yet.

## AI

- Copilot uses the local mock provider when no API key is configured.
- With `CREATE3D_COPILOT_API_KEY`, Copilot calls an OpenAI-compatible chat completions endpoint (`CREATE3D_COPILOT_BASE_URL`, default `https://api.openai.com/v1`; `CREATE3D_COPILOT_MODEL`, default `gpt-4o-mini`).
- Write proposals still require preview and approval before commit.
- AI cannot perform destructive geometry ops through collaboration sync.

## Collaboration

- TCP JSONL sync prototype on localhost.
- Transform/component sync only; create/delete entity blocked remotely.
- No CRDT/OT merge for mesh topology edits.

## Robotics

- In-process mock bridge for synthetic joint states without ROS2 installed.
- TCP sidecar bridge (`create3d-ros2-bridge`) over JSONL IPC; desktop can spawn or connect to an external sidecar.
- Live ROS2 topic subscription (`--ros2`) is reserved for a future release; Beta sidecar mock mode validates the IPC path.

## Plugins

- No stable public plugin ABI.
- Internal Rust crates may change without notice until post-Beta stabilization.

## Packaging

- `cargo run -p xtask -- package` builds release binaries locally.
- CI uploads unsigned Linux binaries on `v*` tags; no signed installers or auto-update channel yet.

## Reporting issues

Use GitHub issue templates under `.github/ISSUE_TEMPLATE/`.
