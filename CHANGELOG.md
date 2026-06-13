# Create3D Beta (0.1.2-beta)

Export polish: GLB textures/UVs and USDA material parity.

## Highlights

- **GLB export materials** — embedded base-color textures and `TEXCOORD_0` when mesh UVs are present
- **USDA export materials** — UsdPreviewSurface / UsdUVTexture with sidecar texture files and UV primvars
- **Release CI** — unchanged; builds Linux release binaries on `v*` tags via `xtask package`

## Build

```bash
cargo run -p xtask -- check
cargo run -p xtask -- package
```

## Tag

```bash
git tag -a v0.1.2-beta -m "Create3D Beta 0.1.2 release"
git push origin v0.1.2-beta
```

## Known limitations

See `Create3D/docs/known-limitations.md`.

---

# Create3D Beta (0.1.1-beta)

Post-Beta update: remote Copilot LLM, ROS2 sidecar IPC, and USDA export.

## Highlights

- **Remote LLM Copilot** — OpenAI-compatible chat completions with typed tool calls (`CREATE3D_COPILOT_API_KEY`, optional `_BASE_URL` / `_MODEL`)
- **ROS2 sidecar skeleton** — `create3d-ros2-bridge` TCP JSONL IPC; desktop **Start Sidecar Bridge**
- **USDA export** — `c3d-export-usd`, CLI `export-usd`, and desktop **Export USD** mesh hierarchy snapshots
- **Release CI** — unchanged; builds Linux release binaries on `v*` tags via `xtask package`

## Build

```bash
cargo run -p xtask -- check
cargo run -p xtask -- package
```

## Tag

```bash
git tag -a v0.1.1-beta -m "Create3D Beta 0.1.1 release"
git push origin v0.1.1-beta
```

## Known limitations

See `Create3D/docs/known-limitations.md`.

---

# Create3D Beta (0.1.0-beta)

Public Beta release after the Beta entry milestone.

## Highlights

- **Open Project** — load any Create3D project directory from the desktop toolbar or command palette
- **GLB export** — `c3d-export-gltf`, CLI `export-gltf`, and desktop **Export GLB** snapshot
- **Copilot API key stub** — `CREATE3D_COPILOT_API_KEY` / desktop field; remote provider still uses mock responses
- **Release CI** — GitHub Actions builds Linux release binaries on `v*` tags via `xtask package`

## Build

```bash
cargo run -p xtask -- check
cargo run -p xtask -- package
```

## Tag

```bash
git tag -a v0.1.0-beta -m "Create3D Beta release"
git push origin v0.1.0-beta
```

## Known limitations

See `Create3D/docs/known-limitations.md`.

---

# Create3D Alpha (0.1.0-alpha)

Public Alpha release after the Month 12 hardening milestone.

## Highlights

- Desktop editor with mesh, point cloud, Gaussian splat, Copilot, robotics, and collaboration panels
- Project templates and sample projects under `samples/`
- Crash-safe autosave/recovery in the desktop editor
- Import errors include source path context
- CLI `create`, `list-templates`, and `bench` commands
- `xtask samples`, `xtask bench`, and `xtask package` developer tasks
- User, plugin, AI tool, and robotics guides

## Tag

```bash
git tag -a v0.1.0-alpha -m "Create3D Alpha release"
git push origin v0.1.0-alpha
```
