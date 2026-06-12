# Security Policy

## Supported versions

Create3D is pre-alpha. Only the latest `main` branch receives security fixes.

## Reporting a vulnerability

Please report security issues privately rather than opening a public GitHub issue.

Include:

- affected component or crate,
- reproduction steps or proof of concept,
- impact assessment,
- suggested mitigation if available.

Do not include sensitive customer data, private keys, IMEI/serial numbers, or full untrusted asset dumps in reports.

## Scope

In scope:

- scene/asset importers,
- plugin/WASM host boundaries,
- collaboration sync server,
- ROS2 sidecar IPC,
- AI tool sandboxing.

Out of scope for initial bootstrap:

- third-party model providers,
- user-authored Python plugins without sandboxing.

## Response expectations

We aim to acknowledge reports within 7 days and provide a remediation plan when possible.
