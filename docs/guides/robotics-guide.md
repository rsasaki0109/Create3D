# Robotics guide (Beta)

Create3D Beta includes URDF import, kinematic hierarchy visualization, in-process mock bridge, TCP sidecar bridge, and TF/joint inspection.

## Import a robot

CLI:

```bash
cargo run -p create3d-cli -- import-urdf \
  --input /path/to/robot.urdf \
  --output /path/to/project.c3d \
  --name my-robot
```

Desktop: **Import URDF** toolbar button or command palette.

Sample project:

```bash
cargo run -p xtask -- samples
# open samples/urdf-robot-scene/
```

## Scene representation

URDF import creates:

- A `RobotRoot` entity
- `RobotLink` entities for each link
- `RobotJoint` entities for joints
- Primitive visuals where URDF geometry is available

## Mock bridge

1. Import or open a URDF scene.
2. Open the **Robotics** panel.
3. Click **Start Mock Bridge**.

The mock bridge publishes synthetic joint states so you can verify live transform updates and TF tree output without ROS2 installed.

## Sidecar bridge (TCP IPC)

Beta ships `create3d-ros2-bridge`, a sidecar process that speaks the same JSONL protocol as the mock bridge over TCP.

Manual sidecar:

```bash
cargo run -p create3d-ros2-bridge -- \
  --listen 127.0.0.1:9741 \
  --robot-name preview_arm \
  --joint-names shoulder,elbow
```

Desktop:

1. Import or open a URDF scene.
2. Open the **Robotics** panel.
3. Click **Start Sidecar Bridge**.

The editor connects to `CREATE3D_ROS2_BRIDGE_ADDR` (default `127.0.0.1:9741`). If nothing is listening, it tries to spawn `CREATE3D_ROS2_BRIDGE_BIN` (default `create3d-ros2-bridge`) with the scene's robot/joint names.

Sidecar mock mode mirrors the in-process mock bridge. A future `--ros2` mode will subscribe to live ROS2 topics in environments with ROS2 installed.

Protocol types live in `robotics/c3d-robotics-core` (`BridgeEnvelope`, `SidecarClient`).

## TF tree

The Robotics panel lists TF frames derived from the imported hierarchy. Use it to confirm parent/child link relationships after import.

## Limitations

See robotics items in `Create3D/docs/known-limitations.md`.
