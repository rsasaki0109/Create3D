# User guide (Beta)

Create3D Beta is a Rust-native 3D editor prototype. SceneDB is authoritative; all edits are typed transactions.

## Quick start

1. Build and verify:

```bash
cargo run -p xtask -- check
```

2. Launch the desktop editor:

```bash
cargo run -p create3d-desktop
```

3. Open a sample project with **Open Project** (toolbar or command palette), or generate samples:

```bash
cargo run -p xtask -- samples
```

4. Export snapshots with **Export GLB** / **Export USD** or:

```bash
cargo run -p create3d-cli -- export-gltf \
  --project Create3D/samples/mesh-scene \
  --output /tmp/mesh-scene.glb

cargo run -p create3d-cli -- export-usd \
  --project Create3D/samples/mesh-scene \
  --output /tmp/mesh-scene.usda
```

## Core workflows

### Scene editing

- Select entities in the hierarchy or viewport.
- Move selection with the gizmo or inspector translation fields.
- Undo/redo is handled by the transaction manager.
- Use **Save Project** in the toolbar to persist changes and clear recovery snapshots.

### Imports

Use toolbar buttons or the command palette (`Ctrl+Shift+P`):

- GLB / glTF meshes
- PLY point clouds (auto-detects 3DGS when applicable)
- 3DGS PLY
- URDF robots

Import failures report the source path and underlying parser error.

### Copilot

Open the Copilot panel and try:

- `how many entities?`
- `what is selected?`
- `move up 1` (requires selection; preview then Approve)
- `rename to Lamp`

Without an API key, Copilot uses the local mock provider. With `CREATE3D_COPILOT_API_KEY`, it calls an OpenAI-compatible chat endpoint (`CREATE3D_COPILOT_BASE_URL`, `CREATE3D_COPILOT_MODEL` optional).

Approved proposals commit as normal scene transactions.

### Robotics

1. Import URDF or open `samples/urdf-robot-scene/`.
2. Open the Robotics panel.
3. Start the mock ROS2 bridge to drive preview joint states.

### Collaboration

1. Start the sync server:

```bash
cargo run -p create3d-sync-server
```

2. Connect from the Collaboration panel in two desktop instances.
3. Supported remote sync: transforms and safe component updates. Create/delete entity sync is blocked by policy.

## Recovery

The desktop editor writes autosave snapshots under `<project>/.recovery/` every 30 seconds while there are unsaved changes. On restart, a recovery banner offers **Restore** or **Dismiss**.

## Project layout

```text
project/
  manifest.c3d.toml
  scenes/main.c3dscene.json
  assets/
  thumbnails/
  collab/          # comments and branch proposals (when used)
  .recovery/       # autosave snapshots
```

## Further reading

- Architecture: `Create3D/docs/architecture/create3d_master_architecture_design.md`
- Known limitations: `Create3D/docs/known-limitations.md`
