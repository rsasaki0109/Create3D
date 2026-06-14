#!/usr/bin/env python3
"""Create3D ROS2 sidecar bridge over TCP JSONL IPC."""

from __future__ import annotations

import argparse
import json
import socket
import socketserver
import sys
import threading
import time
from dataclasses import dataclass, field
from typing import Dict, Iterable, List, Optional, Tuple

PROTOCOL_VERSION = 1
DEFAULT_LISTEN = "127.0.0.1:9741"
DEFAULT_JOINT_STATES_TOPIC = "/joint_states"
DEFAULT_TF_TOPIC = "/tf"
DEFAULT_TF_STATIC_TOPIC = "/tf_static"
DEFAULT_TF_ROOT_FRAME = "base_link"
JOINT_STATE_TYPE = "sensor_msgs/msg/JointState"
TF_MESSAGE_TYPE = "tf2_msgs/msg/TFMessage"


@dataclass
class JointStateSnapshot:
    joint_names: List[str] = field(default_factory=list)
    positions: List[float] = field(default_factory=list)
    lock: threading.Lock = field(default_factory=threading.Lock)

    def update(self, joint_names: List[str], positions: List[float]) -> None:
        with self.lock:
            self.joint_names = joint_names
            self.positions = positions

    def read(self) -> tuple[List[str], List[float]]:
        with self.lock:
            return list(self.joint_names), list(self.positions)


@dataclass
class TfSnapshot:
    root_frame: str = DEFAULT_TF_ROOT_FRAME
    edges: Dict[Tuple[str, str], dict] = field(default_factory=dict)
    lock: threading.Lock = field(default_factory=threading.Lock)

    def upsert(self, transforms: Iterable[object]) -> None:
        with self.lock:
            for transform in transforms:
                parent = normalize_frame(transform.header.frame_id)
                child = normalize_frame(transform.child_frame_id)
                self.edges[(parent, child)] = geometry_transform_to_dict(transform.transform)

    def read_tree(self) -> tuple[str, List[dict]]:
        with self.lock:
            edges = [
                {
                    "parent": parent,
                    "child": child,
                    "transform": payload,
                }
                for (parent, child), payload in sorted(self.edges.items())
            ]
            return self.root_frame, edges


def normalize_frame(frame_id: str) -> str:
    return frame_id.lstrip("/")


def geometry_transform_to_dict(transform: object) -> dict:
    translation = transform.translation
    rotation = transform.rotation
    return {
        "translation": [
            float(translation.x),
            float(translation.y),
            float(translation.z),
        ],
        "rotation": [
            float(rotation.x),
            float(rotation.y),
            float(rotation.z),
            float(rotation.w),
        ],
        "scale": [1.0, 1.0, 1.0],
    }


def envelope(message: dict) -> dict:
    return {"version": PROTOCOL_VERSION, "message": message}


def topic_list_message(
    joint_topic: str,
    include_tf: bool,
    tf_topic: str,
) -> dict:
    topics = [
        {"name": joint_topic, "message_type": JOINT_STATE_TYPE},
    ]
    if include_tf:
        topics.append({"name": tf_topic, "message_type": TF_MESSAGE_TYPE})
    return envelope({"type": "topic_list", "topics": topics})


def joint_state_message(topic: str, joint_names: List[str], positions: List[float]) -> dict:
    return envelope(
        {
            "type": "joint_state",
            "topic": topic,
            "joint_names": joint_names,
            "positions": positions,
        }
    )


def tf_tree_message(root_frame: str, edges: List[dict]) -> dict:
    return envelope(
        {
            "type": "tf_tree",
            "root_frame": root_frame,
            "edges": edges,
        }
    )


def filter_joint_state(
    msg_names: Iterable[str],
    msg_positions: Iterable[float],
    filter_names: List[str],
) -> tuple[List[str], List[float]]:
    if not filter_names:
        return list(msg_names), [float(value) for value in msg_positions]

    lookup = {
        name: float(position) for name, position in zip(msg_names, msg_positions)
    }
    names: List[str] = []
    positions: List[float] = []
    for name in filter_names:
        if name not in lookup:
            continue
        names.append(name)
        positions.append(lookup[name])
    return names, positions


def write_json_line(stream: socket.socket, payload: dict) -> None:
    line = json.dumps(payload, separators=(",", ":"))
    stream.sendall(f"{line}\n".encode("utf-8"))


def serve_client(
    conn: socket.socket,
    addr: tuple[str, int],
    joint_topic: str,
    tf_topic: str,
    include_tf: bool,
    filter_names: List[str],
    joint_snapshot: JointStateSnapshot,
    tf_snapshot: Optional[TfSnapshot],
    tick_ms: int,
) -> None:
    print(f"sidecar client connected from {addr[0]}:{addr[1]}", flush=True)
    conn_file = conn.makefile("rb")
    try:
        hello = conn_file.readline()
        if hello.strip():
            print(f"sidecar hello: {hello.decode('utf-8', errors='replace').strip()}", flush=True)
    except OSError:
        return

    try:
        write_json_line(conn, topic_list_message(joint_topic, include_tf, tf_topic))
        while True:
            joint_names, positions = joint_snapshot.read()
            if joint_names and positions:
                write_json_line(
                    conn,
                    joint_state_message(joint_topic, joint_names, positions),
                )
            if include_tf and tf_snapshot is not None:
                root_frame, edges = tf_snapshot.read_tree()
                if edges:
                    write_json_line(conn, tf_tree_message(root_frame, edges))
            time.sleep(tick_ms / 1000.0)
    except (BrokenPipeError, ConnectionResetError, OSError):
        print(f"sidecar client disconnected from {addr[0]}:{addr[1]}", flush=True)
    finally:
        conn.close()


class SidecarServer(socketserver.ThreadingMixIn, socketserver.TCPServer):
    allow_reuse_address = True
    daemon_threads = True


def parse_listen(listen: str) -> tuple[str, int]:
    host, _, port_text = listen.rpartition(":")
    if not host:
        host = "127.0.0.1"
    return host, int(port_text)


def run_tcp_server(
    listen: str,
    joint_topic: str,
    tf_topic: str,
    include_tf: bool,
    filter_names: List[str],
    joint_snapshot: JointStateSnapshot,
    tf_snapshot: Optional[TfSnapshot],
    tick_ms: int,
) -> None:
    host, port = parse_listen(listen)

    class Handler(socketserver.BaseRequestHandler):
        def handle(self) -> None:
            serve_client(
                self.request,
                self.client_address,
                joint_topic,
                tf_topic,
                include_tf,
                filter_names,
                joint_snapshot,
                tf_snapshot,
                tick_ms,
            )

    with SidecarServer((host, port), Handler) as server:
        print(
            f"Create3D ROS2 sidecar listening on {host}:{port} "
            f"(joints={joint_topic}, tf={include_tf}, filter={filter_names or 'all'})",
            flush=True,
        )
        server.serve_forever()


def run_ros2_subscription(
    joint_topic: str,
    tf_topic: str,
    tf_static_topic: str,
    include_tf: bool,
    tf_root_frame: str,
    filter_names: List[str],
    joint_snapshot: JointStateSnapshot,
    tf_snapshot: Optional[TfSnapshot],
) -> None:
    try:
        import rclpy
        from rclpy.node import Node
        from sensor_msgs.msg import JointState
        from tf2_msgs.msg import TFMessage
    except ImportError as err:
        print(
            "Failed to import rclpy/sensor_msgs/tf2_msgs. Source a ROS2 workspace first, e.g.\n"
            "  source /opt/ros/jazzy/setup.bash",
            file=sys.stderr,
        )
        raise SystemExit(1) from err

    class BridgeNode(Node):
        def __init__(self) -> None:
            super().__init__("create3d_ros2_bridge")
            self.create_subscription(JointState, joint_topic, self.on_joint_state, 10)
            if include_tf and tf_snapshot is not None:
                self.create_subscription(TFMessage, tf_topic, self.on_tf, 10)
                self.create_subscription(TFMessage, tf_static_topic, self.on_tf, 10)
                tf_snapshot.root_frame = tf_root_frame
            self.get_logger().info(
                f"subscribed to {joint_topic}"
                + (f", {tf_topic}, {tf_static_topic}" if include_tf else "")
            )

        def on_joint_state(self, msg: JointState) -> None:
            names, positions = filter_joint_state(msg.name, msg.position, filter_names)
            if not names:
                return
            joint_snapshot.update(names, positions)

        def on_tf(self, msg: TFMessage) -> None:
            if tf_snapshot is None:
                return
            tf_snapshot.upsert(msg.transforms)

    rclpy.init()
    node = BridgeNode()
    try:
        rclpy.spin(node)
    finally:
        node.destroy_node()
        rclpy.shutdown()


def main() -> None:
    parser = argparse.ArgumentParser(description="Create3D ROS2 sidecar bridge")
    parser.add_argument("--listen", default=DEFAULT_LISTEN)
    parser.add_argument("--joint-states-topic", default=DEFAULT_JOINT_STATES_TOPIC)
    parser.add_argument("--tf-topic", default=DEFAULT_TF_TOPIC)
    parser.add_argument("--tf-static-topic", default=DEFAULT_TF_STATIC_TOPIC)
    parser.add_argument("--tf-root-frame", default=DEFAULT_TF_ROOT_FRAME)
    parser.add_argument(
        "--joint-names",
        default="",
        help="Comma-separated joint names to forward (default: all from ROS2 message)",
    )
    parser.add_argument(
        "--no-tf",
        action="store_true",
        help="Disable live TF forwarding",
    )
    parser.add_argument("--tick-ms", type=int, default=50)
    args = parser.parse_args()

    filter_names = [name.strip() for name in args.joint_names.split(",") if name.strip()]
    include_tf = not args.no_tf
    joint_snapshot = JointStateSnapshot()
    tf_snapshot = TfSnapshot(root_frame=args.tf_root_frame) if include_tf else None

    ros_thread = threading.Thread(
        target=run_ros2_subscription,
        args=(
            args.joint_states_topic,
            args.tf_topic,
            args.tf_static_topic,
            include_tf,
            args.tf_root_frame,
            filter_names,
            joint_snapshot,
            tf_snapshot,
        ),
        daemon=True,
    )
    ros_thread.start()

    # Give rclpy a moment to start before accepting clients.
    time.sleep(0.5)
    run_tcp_server(
        args.listen,
        args.joint_states_topic,
        args.tf_topic,
        include_tf,
        filter_names,
        joint_snapshot,
        tf_snapshot,
        args.tick_ms,
    )


if __name__ == "__main__":
    main()
