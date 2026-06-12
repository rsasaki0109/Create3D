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

## Build

```bash
cargo run -p xtask -- check
cargo run -p xtask -- package
```

## Tag

```bash
git tag -a v0.1.0-alpha -m "Create3D Alpha release"
git push origin v0.1.0-alpha
```

## Known limitations

See `Create3D/docs/known-limitations.md`.
