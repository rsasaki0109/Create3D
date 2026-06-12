# Sample projects

These projects are generated from built-in templates and can be opened with the desktop editor or inspected on disk.

## Regenerate

```bash
cargo run -p xtask -- samples
```

## Open in desktop

Point the desktop editor at a sample directory, or copy a sample into your temp project path:

```bash
cp -a samples/mesh-scene ~/.cache/create3d-desktop-project
cargo run -p create3d-desktop
```

## Included samples

| Directory | Template | Purpose |
|-----------|----------|---------|
| `mesh-scene/` | mesh-scene | Floor plane + unit cube |
| `point-cloud-scene/` | point-cloud-scene | Synthetic chunked point cloud |
| `gaussian-splat-scene/` | gaussian-splat-scene | Synthetic Gaussian splats |
| `urdf-robot-scene/` | urdf-robot-scene | Preview URDF arm hierarchy |
| `ai-editing-demo/` | ai-editing-demo | Copilot move/rename demo entity |

Create a fresh project from any template:

```bash
cargo run -p create3d-cli -- create \
  --output /tmp/my-project \
  --name "My Project" \
  --template mesh-scene
```
