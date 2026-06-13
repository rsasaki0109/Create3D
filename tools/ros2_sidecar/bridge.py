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
from typing import Iterable, List, Optional

PROTOCOL_VERSION = 1
DEFAULT_LISTEN = "127.0.0.1:9741"
DEFAULT_JOINT_STATES_TOPIC = "/joint_states"
JOINT_STATE_TYPE = "sensor_msgs/msg/JointState"


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


def envelope(message: dict) -> dict:
    return {"version": PROTOCOL_VERSION, "message": message}


def topic_list_message(topic: str) -> dict:
    return envelope(
        {
            "type": "topic_list",
            "topics": [{"name": topic, "message_type": JOINT_STATE_TYPE}],
        }
    )


def joint_state_message(topic: str, joint_names: List[str], positions: List[float]) -> dict:
    return envelope(
        {
            "type": "joint_state",
            "topic": topic,
            "joint_names": joint_names,
            "positions": positions,
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
    topic: str,
    filter_names: List[str],
    snapshot: JointStateSnapshot,
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
        write_json_line(conn, topic_list_message(topic))
        while True:
            joint_names, positions = snapshot.read()
            if joint_names and positions:
                write_json_line(
                    conn,
                    joint_state_message(topic, joint_names, positions),
                )
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
    topic: str,
    filter_names: List[str],
    snapshot: JointStateSnapshot,
    tick_ms: int,
) -> None:
    host, port = parse_listen(listen)

    class Handler(socketserver.BaseRequestHandler):
        def handle(self) -> None:
            serve_client(
                self.request,
                self.client_address,
                topic,
                filter_names,
                snapshot,
                tick_ms,
            )

    with SidecarServer((host, port), Handler) as server:
        print(
            f"Create3D ROS2 sidecar listening on {host}:{port} "
            f"(topic={topic}, joints={filter_names or 'all'})",
            flush=True,
        )
        server.serve_forever()


def run_ros2_subscription(
    topic: str,
    filter_names: List[str],
    snapshot: JointStateSnapshot,
) -> None:
    try:
        import rclpy
        from rclpy.node import Node
        from sensor_msgs.msg import JointState
    except ImportError as err:
        print(
            "Failed to import rclpy/sensor_msgs. Source a ROS2 workspace first, e.g.\n"
            "  source /opt/ros/jazzy/setup.bash",
            file=sys.stderr,
        )
        raise SystemExit(1) from err

    class BridgeNode(Node):
        def __init__(self) -> None:
            super().__init__("create3d_ros2_bridge")
            self.subscription = self.create_subscription(
                JointState,
                topic,
                self.on_joint_state,
                10,
            )
            self.get_logger().info(f"subscribed to {topic}")

        def on_joint_state(self, msg: JointState) -> None:
            names, positions = filter_joint_state(msg.name, msg.position, filter_names)
            if not names:
                return
            snapshot.update(names, positions)

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
    parser.add_argument(
        "--joint-names",
        default="",
        help="Comma-separated joint names to forward (default: all from ROS2 message)",
    )
    parser.add_argument("--tick-ms", type=int, default=50)
    args = parser.parse_args()

    filter_names = [name.strip() for name in args.joint_names.split(",") if name.strip()]
    snapshot = JointStateSnapshot()

    ros_thread = threading.Thread(
        target=run_ros2_subscription,
        args=(args.joint_states_topic, filter_names, snapshot),
        daemon=True,
    )
    ros_thread.start()

    # Give rclpy a moment to start before accepting clients.
    time.sleep(0.5)
    run_tcp_server(args.listen, args.joint_states_topic, filter_names, snapshot, args.tick_ms)


if __name__ == "__main__":
    main()
