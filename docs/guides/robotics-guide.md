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
- Primitive visuals (box/cylinder/sphere) and external mesh references (`.stl`, `.glb`, `.gltf`)

### External mesh paths

URDF `<mesh filename="...">` references are resolved from the URDF file directory:

- Relative paths: `meshes/base.stl`
- Package URIs: `package://my_robot/meshes/base.stl` (path after the package name is resolved from the URDF directory or its parent)
- File URIs: `file:///absolute/path/to/mesh.stl`

Place mesh files next to the URDF or in the usual `../meshes/` layout. Collada (`.dae`) is not supported yet.

## Mock bridge

1. Import or open a URDF scene.
2. Open the **Robotics** panel.
3. Click **Start Mock Bridge**.

The mock bridge publishes synthetic joint states so you can verify live transform updates and TF tree output without ROS2 installed.

## Sidecar bridge (TCP IPC)

Beta ships `create3d-ros2-bridge`, a sidecar process that speaks the same JSONL protocol as the mock bridge over TCP.

### Mock sidecar

```bash
cargo run -p create3d-ros2-bridge -- \
  --listen 127.0.0.1:9741 \
  --robot-name preview_arm \
  --joint-names shoulder,elbow
```

### Live ROS2 sidecar

Requires a sourced ROS2 environment with `rclpy` and a publisher on `/joint_states` (or your topic).

```bash
source /opt/ros/jazzy/setup.bash

cargo run -p create3d-ros2-bridge -- \
  --ros2 \
  --no-mock \
  --listen 127.0.0.1:9741 \
  --joint-names shoulder,elbow \
  --joint-states-topic /joint_states \
  --tf-root-frame base_link
```

This delegates to `tools/ros2_sidecar/bridge.py`. Override the script or Python interpreter when needed:

- `CREATE3D_ROS2_BRIDGE_PY=/path/to/bridge.py`
- `CREATE3D_ROS2_BRIDGE_PYTHON=python3`
- `CREATE3D_ROS2_TF_TOPIC=/tf`
- `CREATE3D_ROS2_TF_STATIC_TOPIC=/tf_static`
- `CREATE3D_ROS2_TF_ROOT=base_link`
- `CREATE3D_ROS2_BRIDGE_NO_TF=1` to disable TF forwarding

Or run the Python bridge directly:

```bash
python3 Create3D/tools/ros2_sidecar/bridge.py \
  --listen 127.0.0.1:9741 \
  --joint-names shoulder,elbow
```

### Desktop

1. Import or open a URDF scene.
2. Open the **Robotics** panel.
3. Click **Start Sidecar Bridge**.

The editor connects to `CREATE3D_ROS2_BRIDGE_ADDR` (default `127.0.0.1:9741`). If nothing is listening, it tries to spawn `CREATE3D_ROS2_BRIDGE_BIN` (default `create3d-ros2-bridge`) with the scene's robot/joint names.

For live ROS2 from the desktop spawn path, set:

```bash
export CREATE3D_ROS2_BRIDGE_ROS2=1
export CREATE3D_ROS2_JOINT_STATES_TOPIC=/joint_states
export CREATE3D_ROS2_TF_ROOT=base_link
```

Sidecar mock mode mirrors the in-process mock bridge. ROS2 mode forwards live joint states and TF snapshots from the configured topics.

## TF tree

The Robotics panel lists TF frames from live ROS2 when a sidecar bridge is connected. Otherwise it shows frames derived from the imported URDF hierarchy. Live TF updates scene link transforms when child frame names match imported link names.

## Limitations

See robotics items in `Create3D/docs/known-limitations.md`.
