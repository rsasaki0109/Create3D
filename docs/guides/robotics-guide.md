# Robotics guide (Alpha)

Create3D Alpha includes URDF import, kinematic hierarchy visualization, a mock ROS2 bridge, and TF/joint inspection.

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

## Real ROS2 bridge (optional)

The architecture targets a sidecar ROS2 bridge over IPC. Alpha ships the protocol types in `c3d-robotics-core` and mock tests; production ROS2 wiring is environment-specific and not required for Alpha acceptance.

## TF tree

The Robotics panel lists TF frames derived from the imported hierarchy. Use it to confirm parent/child link relationships after import.

## Limitations

See robotics items in `Create3D/docs/known-limitations.md`.
