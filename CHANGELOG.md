# Create3D Beta (0.1.9-beta)

URDF Collada (`.dae`) mesh import v0 for triangle Collada geometry referenced from URDF visuals.

## Highlights

- **Collada import** — new `c3d-import-collada` crate parses triangle `<triangles>` and triangle-only `<polylist>` geometry
- **URDF wiring** — `load_urdf_mesh_file()` accepts `.dae` alongside `.stl`, `.glb`, and `.gltf`
- **Import tests** — project importer covers relative `.dae` references next to the URDF file

## Build

```bash
cargo run -p xtask -- check
cargo run -p xtask -- package
```

## Tag

```bash
git tag -a v0.1.9-beta -m "Create3D Beta 0.1.9 release"
git push origin v0.1.9-beta
```

## Known limitations

See `Create3D/docs/known-limitations.md`.

---

# Create3D Beta (0.1.8-beta)

Binary PLY import/export v0: round-trip point cloud snapshots in binary little-endian format.

## Highlights

- **Binary PLY import** — shared header parser in `c3d-import-ply`; reads `float` + `uchar` vertex properties from binary little-endian files
- **Binary PLY export** — `c3d-export-ply` defaults to binary little-endian; ASCII available via CLI `--ascii`
- **Round-trip** — export binary PLY from scene entities and re-import with existing chunking pipeline

## Build

```bash
cargo run -p xtask -- check
cargo run -p xtask -- package
```

## Tag

```bash
git tag -a v0.1.8-beta -m "Create3D Beta 0.1.8 release"
git push origin v0.1.8-beta
```

## Known limitations

See `Create3D/docs/known-limitations.md`.

---

# Create3D Beta (0.1.7-beta)

3DGS PLY export: ASCII Gaussian splat snapshots from scene splat entities.

## Highlights

- **3DGS export** — new `c3d-export-gsplat` crate merges scene `GaussianSplatRef` entities into one 3DGS PLY
- **Render settings aware** — applies entity transforms, crop filters, opacity scale, and size scale before writing
- **CLI + Desktop** — `export-gsplat` subcommand and **Export 3DGS** toolbar / command palette action

## Build

```bash
cargo run -p xtask -- check
cargo run -p xtask -- package
```

## Tag

```bash
git tag -a v0.1.7-beta -m "Create3D Beta 0.1.7 release"
git push origin v0.1.7-beta
```

## Known limitations

See `Create3D/docs/known-limitations.md`.

---

# Create3D Beta (0.1.6-beta)

Live ROS2 TF forwarding: sidecar publishes `tf_tree` snapshots from `/tf` and `/tf_static`.

## Highlights

- **Live TF sidecar** — `bridge.py` subscribes to `/tf` + `/tf_static`, emits `TfTreeMessage` over JSONL IPC
- **Desktop integration** — Robotics panel shows live TF tree; matching URDF link frames update scene transforms
- **Config** — `CREATE3D_ROS2_TF_TOPIC`, `CREATE3D_ROS2_TF_STATIC_TOPIC`, `CREATE3D_ROS2_TF_ROOT`, `CREATE3D_ROS2_BRIDGE_NO_TF=1`

## Build

```bash
cargo run -p xtask -- check
cargo run -p xtask -- package
```

## Tag

```bash
git tag -a v0.1.6-beta -m "Create3D Beta 0.1.6 release"
git push origin v0.1.6-beta
```

## Known limitations

See `Create3D/docs/known-limitations.md`.

---

# Create3D Beta (0.1.5-beta)

Point cloud PLY export: ASCII snapshots from scene point cloud entities.

## Highlights

- **PLY export** — new `c3d-export-ply` crate merges scene `PointCloudRef` entities into one ASCII PLY snapshot
- **World/crop aware** — applies entity transforms and per-entity crop filters before writing vertices
- **CLI + Desktop** — `export-ply` subcommand and **Export PLY** toolbar / command palette action

## Build

```bash
cargo run -p xtask -- check
cargo run -p xtask -- package
```

## Tag

```bash
git tag -a v0.1.5-beta -m "Create3D Beta 0.1.5 release"
git push origin v0.1.5-beta
```

## Known limitations

See `Create3D/docs/known-limitations.md`.

---

# Create3D Beta (0.1.4-beta)

URDF external mesh import: resolve `package://` and relative paths; load STL and GLB/GLTF visuals.

## Highlights

- **URDF mesh paths** — `resolve_urdf_mesh_path` handles `package://`, `file://`, relative, and absolute references from the URDF directory
- **External mesh formats** — new `c3d-import-stl` crate; URDF `<mesh>` visuals load `.stl`, `.glb`, and `.gltf`
- **Import tests** — project importer covers relative STL references next to the URDF file

## Build

```bash
cargo run -p xtask -- check
cargo run -p xtask -- package
```

## Tag

```bash
git tag -a v0.1.4-beta -m "Create3D Beta 0.1.4 release"
git push origin v0.1.4-beta
```

## Known limitations

See `Create3D/docs/known-limitations.md`.

---

# Create3D Beta (0.1.3-beta)

Live ROS2 sidecar: Python `rclpy` bridge for real `/joint_states` over TCP JSONL IPC.

## Highlights

- **Live ROS2 sidecar** — `create3d-ros2-bridge --ros2 --no-mock` delegates to `tools/ros2_sidecar/bridge.py` (`rclpy` subscription, joint name filtering)
- **Desktop integration** — set `CREATE3D_ROS2_BRIDGE_ROS2=1` to spawn live mode; `CREATE3D_ROS2_JOINT_STATES_TOPIC` for topic override
- **Mock sidecar unchanged** — default TCP mock bridge still works without ROS2 installed

## Build

```bash
cargo run -p xtask -- check
cargo run -p xtask -- package
```

## Tag

```bash
git tag -a v0.1.3-beta -m "Create3D Beta 0.1.3 release"
git push origin v0.1.3-beta
```

## Known limitations

See `Create3D/docs/known-limitations.md`.

---

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
