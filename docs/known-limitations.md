# Known limitations (Alpha)

Create3D **0.1.0-alpha** is a public prototype. Expect rough edges.

## Editor

- Desktop persists the demo project under the system temp directory by default.
- No multi-document UI; one project per editor instance.
- Undo/redo covers scene transactions but not asset blob history.
- UI polish is functional, not production-grade.

## Rendering

- wgpu backend only; advanced native backends are not shipped.
- Large mesh scenes are not fully optimized; point clouds and splats use chunk residency but not full streaming LOD.

## Import

- glTF: supported paths only; exotic extensions may fail with path-qualified errors.
- PLY / 3DGS: ASCII-oriented importers; binary PLY support is limited.
- URDF: preview arm and common links; mesh packages with external assets may need manual path fixes.

## AI

- Mock local Copilot provider only.
- No remote model integration in Alpha.
- AI cannot perform destructive geometry ops through collaboration sync.

## Collaboration

- TCP JSONL sync prototype on localhost.
- Transform/component sync only; create/delete entity blocked remotely.
- No CRDT/OT merge for mesh topology edits.

## Robotics

- Mock bridge only in default Alpha workflow.
- Real ROS2 sidecar is architecture-ready but not bundled as a turnkey installer.

## Plugins

- No stable public plugin ABI.
- Internal Rust crates may change without notice until post-Alpha stabilization.

## Packaging

- `cargo run -p xtask -- package` builds release binaries locally.
- No signed installers or auto-update channel in Alpha.

## Reporting issues

Use GitHub issue templates under `.github/ISSUE_TEMPLATE/`.
