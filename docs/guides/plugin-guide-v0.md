# Plugin guide v0 (Alpha)

Create3D Alpha does not ship a stable public plugin ABI yet. This guide describes the supported extension directions for early adopters.

## Supported extension models

1. **Sidecar processes** — robotics bridge, future importers/exporters, automation services communicating over documented IPC/JSON protocols.
2. **WASM plugins (planned)** — sandboxed tools and panels; not yet wired in Alpha.
3. **Internal Rust crates** — workspace crates such as `c3d-import-gltf` are development modules, not a semver-stable plugin surface.

Do not depend on internal crate layouts across Alpha releases.

## Integration points today

| Area | Extension approach |
|------|-------------------|
| Scene edits | Typed `SceneOperation` / `Transaction` via tools or sidecars |
| AI tools | Register tools in `c3d-ai-tool-protocol` with permission checks |
| Import | Add a crate under `asset/` and call from `c3d-project` |
| Rendering | Implement `c3d-rhi` traits in a backend crate |
| Collaboration | `c3d-sync` protocol messages and hub policy hooks |

## Dependency rules

Read `Create3D/docs/architecture/dependency-rules.md` before adding crates.

Key constraints:

- `c3d-core` stays free of editor/renderer/AI dependencies.
- SceneDB (`c3d-scene-doc`) remains authoritative over ECS.
- Renderer code depends on `c3d-rhi`, not wgpu directly (desktop bootstrap excepted).

## Recommended path for external plugins

1. Prototype as a sidecar CLI/service with JSON or newline-delimited JSON IPC.
2. Propose an RFC in `docs/rfcs/` if you need in-process integration.
3. Wait for WASM plugin host APIs before shipping user-installable editor plugins.

## Versioning

Alpha releases may break internal APIs. Track `C3D_API_VERSION` in `c3d-core` for schema-level compatibility only.
