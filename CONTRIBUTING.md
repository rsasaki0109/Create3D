# Contributing to Create3D

Thank you for contributing to Create3D.

## Development setup

1. Install Rust 1.88+ with `rustfmt` and `clippy`.
2. Clone the repository.
3. Run `cargo run -p xtask -- check`.

Useful commands:

```bash
cargo run -p xtask -- check
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

## Architecture rules

Before adding crates or dependencies, read:

- `Create3D/docs/architecture/dependency-rules.md`

Key rules:

1. `c3d-core` must not depend on editor, renderer, AI, or robotics crates.
2. SceneDB is authoritative; ECS is derived.
3. Public plugins should target WASM or sidecar APIs, not unstable internal crates.

## RFC process

Significant architecture changes should go through an RFC in `docs/rfcs/`.

1. Copy `docs/rfcs/0000-template.md`.
2. Open a PR with the RFC for discussion.
3. Implement after acceptance.

## Pull requests

- Keep changes focused.
- Add tests for behavior changes.
- Run `cargo run -p xtask -- check` before opening a PR.
- Use conventional commit subjects when possible.

## Code of conduct

Be respectful, precise, and evidence-driven. Separate confirmed facts from inference in investigation notes.

## Security

Report security issues as described in `SECURITY.md`. Do not open public issues for exploitable vulnerabilities.
