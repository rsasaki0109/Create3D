# Create3D Master Architecture Design Document

**Project codename:** Create3D  
**Status:** Initial OSS architecture proposal  
**Date:** 2026-06-12  
**Primary positioning:** AI Native / GPU Native / Cloud Native / Rust Native 3D creation platform  
**Product metaphors:** “Figma for 3D”, “VSCode for 3D”, “AI Agent Native 3D Studio”

> Naming note: “Create3D” is treated as a codename in this document. The public OSS brand should not be finalized until GitHub organization/repository availability, major domains, package names, trademarks, and search collision risk are checked. Current public search already shows many “Create3D / Create 3D / CREAT3D” uses in 3D printing, CAD, AI 3D generation, and training businesses.

---

## 0. Executive Summary

Create3D should not be a Blender clone. It should be a new class of 3D studio where the primary unit of work is not a local binary scene file edited through thousands of manual UI commands, but a **semantic, versioned, collaborative, GPU-streamable scene document** that can be edited by humans, scripts, robots, and AI agents through the same transaction system.

Create3D exists because 3D content creation is shifting from:

- manual polygon editing to intent-driven editing,
- local desktop files to collaborative cloud workspaces,
- CPU-side scene traversal to GPU-resident data pipelines,
- mesh-only assets to mixed representations: mesh, B-Rep, NURBS, SDF, point cloud, Gaussian splat, neural fields,
- artist-only workflows to robotics, digital twin, simulation, synthetic data, and generated worlds.

The core architectural bet is:

**Create3D = Scene Database + GPU Data Engine + AI Tool Runtime + Collaborative Editor + Robotics/Digital Twin Bridge.**

The editor is just one client of the system. The CLI, cloud worker, headless renderer, ROS2 bridge, AI agent, and plugin runtime must operate on the same scene operations and asset graph.

### 0.1 Non-negotiable design decisions

1. **Rust-first core.** Memory safety, high concurrency, strong module boundaries, and WebAssembly compatibility matter more than matching legacy DCC internals.
2. **SceneDB is authoritative; ECS is a runtime projection.** A DCC authoring scene needs history, schema evolution, references, collaboration, variants, and undo. ECS alone is not the scene file.
3. **All edits are transactions.** UI tools, AI agents, procedural nodes, robotics bridge updates, and plugins must mutate scenes through typed operations that are undoable, auditable, permissioned, and synchronizable.
4. **Renderer is a GPU data platform.** Rendering, point cloud filtering, Gaussian splat sorting, mesh processing, thumbnails, selection, physics broadphase, and AI-generated previews are GPU jobs scheduled through a render/compute graph.
5. **Collaboration is first-class.** Presence, comments, branches, scene diffs, asset provenance, and conflict-aware edits are part of the document model, not a later SaaS wrapper.
6. **AI agents are tool users, not hidden magical mutators.** Every agent action is a typed tool call that creates a scene transaction, validation result, preview, and provenance record.
7. **Robotics and digital twin are first-class.** ROS2, URDF, MJCF, USD, Autoware, Nav2, live sensor streams, and simulation clocks must be part of the architecture from the beginning.

---

## 1. Vision

### 1.1 Why Create3D should exist

Existing DCC and game-engine editors are powerful, but they were mostly designed before the current AI-native, cloud-native, massive-scene, GPU-compute era. Their internal models often assume a local workstation, a human operating a complex UI, a mostly mesh-centric asset pipeline, and a relatively static distinction between “authoring tool” and “runtime engine.”

Create3D should exist to solve a different problem:

> Let humans and AI agents co-author massive, heterogeneous 3D worlds in real time, with a simple editor, versioned scene operations, GPU-native rendering/processing, and robotics/digital-twin connectivity.

### 1.2 Product identity

Create3D is three products sharing one architecture:

1. **Figma for 3D**  
   Real-time collaboration, comments, branches, presence, shared assets, cloud previews, permissions, and web-accessible review.

2. **VSCode for 3D**  
   A modular, extensible workbench with plugins, command palette, extensions, task runner, integrated terminal/logs, AI tools, and project workspaces.

3. **AI Agent Native 3D Studio**  
   A tool environment where agents can inspect scenes, plan edits, call typed tools, generate assets, validate results, and propose or commit changes.

### 1.3 Redefinition of 3D creation in the AI era

Traditional DCC:

```text
User learns tool → user issues UI command → scene mutates → file saved locally
```

Create3D:

```text
Human or agent expresses intent
→ system builds scene context
→ planner proposes typed operations
→ tools execute into a transaction
→ validators check constraints
→ renderer/simulator previews result
→ user or policy commits
→ transaction syncs to workspace
```

The key shift is from **command execution** to **intent compilation**.

Examples:

- “Turn this warehouse scan into a navigable robot test scene.”
- “Replace all temporary crates with optimized instanced assets and keep forklift clearance.”
- “Generate a 20-second camera move showing assembly line flow.”
- “Convert this room point cloud into walls, floor, doors, and collision volumes.”
- “Import this URDF, attach sensors, connect to /tf and /joint_states, and show Nav2 costmaps.”

Create3D’s value is not just generating geometry. Its value is converting high-level intent into **structured scene changes** with validation, provenance, and collaboration.

### 1.4 Differentiation from Blender

Blender is a powerful, free, open-source 3D creation suite covering modeling, animation, simulation, rendering, compositing, and more. Create3D must not compete by trying to reproduce every Blender feature.

Create3D differentiates by:

- making collaboration and cloud scenes core architecture rather than add-ons,
- using a typed transactional SceneDB rather than a primarily local monolithic scene file model,
- treating AI agents as first-class scene editors,
- designing for heterogeneous geometry from day one: mesh, B-Rep, NURBS, SDF, point cloud, Gaussian splat,
- targeting robotics and digital twin workflows as core verticals,
- exposing an extension model closer to VSCode than to a monolithic application plugin API,
- making GPU compute and streaming scene data central to all large-scene workflows.

Blender interop should be excellent. Blender replacement should not be the Phase 1 goal.

### 1.5 Differentiation from Maya

Maya is a mature professional tool for character animation, VFX, modeling, simulation, and rendering. Create3D should not attempt to beat Maya’s decades of rigging and animation pipeline depth immediately.

Create3D differentiates by:

- being OSS-first rather than a commercial vertical application,
- being Rust-native and cloud-ready,
- exposing scene operations as typed transactions usable by agents,
- prioritizing lightweight onboarding and collaborative workflows,
- focusing on procedural/AI/robotics/digital-twin pipelines where traditional film pipelines are not the only target,
- avoiding a hard dependency on Python as the primary execution substrate.

### 1.6 Differentiation from Houdini

Houdini is the benchmark for procedural node-based 3D/VFX workflows. Create3D should respect Houdini’s procedural depth, not try to clone it.

Create3D differentiates by:

- combining procedural graphs with AI planning and natural-language operations,
- making procedural results part of a collaborative SceneDB with transaction history,
- supporting interactive cloud collaboration and web review as a baseline,
- supporting robotics and digital twin semantics, not just procedural VFX,
- providing a lower-friction editor for scene assembly, point clouds, Gaussian splats, and AI-generated worlds.

Create3D’s procedural system should be practical, incremental, and agent-addressable rather than only artist-authored node networks.

### 1.7 Differentiation from Unity

Unity is primarily a real-time engine/editor for building and deploying games and interactive applications across platforms. Create3D is not a game engine competitor first.

Create3D differentiates by:

- focusing on authoring and operating large 3D scene documents rather than shipping app runtimes,
- making AI-assisted modeling, scene editing, and robotics scene preparation first-class,
- supporting non-game data representations such as point clouds, Gaussian splats, B-Rep, URDF, MJCF, USD, and geospatial/digital-twin assets,
- offering a more DCC-like but simpler creation experience,
- providing cloud-native collaboration and scene operation history as core primitives.

Create3D should export to Unity, not try to replace Unity runtime deployment in Phase 1.

### 1.8 Differentiation from Unreal Engine

Unreal Engine is a high-end real-time 3D creation and runtime platform with world-class rendering, virtual production, and game development capabilities. Create3D should not try to out-Unreal Unreal.

Create3D differentiates by:

- being lighter, more modular, and OSS-native,
- using Rust and a clean plugin/tool protocol rather than a large C++ engine monolith,
- prioritizing AI-agent-native authoring and scene transactions,
- supporting cloud collaboration and web-accessible review from the architecture level,
- focusing on robotics/digital twin and heterogeneous 3D data pipelines,
- offering a simpler editor that can become a universal 3D workbench.

Create3D should interoperate with Unreal via USD/glTF/exporters and eventually plugin bridges.

### 1.9 Target users

Create3D should serve several overlapping personas:

- **AI-native 3D creators:** prompt, edit, iterate, publish.
- **Technical artists:** procedural generation, scene optimization, materials, asset pipelines.
- **Robotics engineers:** URDF/MJCF import, ROS2 live visualization, Nav2/Autoware integration.
- **Digital twin engineers:** point clouds, sensors, facility scenes, simulation.
- **Game/world builders:** quick environment creation and export.
- **3D AI researchers:** plug custom text-to-3D, reconstruction, segmentation, Gaussian rendering, neural rendering.
- **OSS developers:** extend the editor, renderer, importers, AI tools, and cloud services.

### 1.10 Product principles

1. **Simple surface, deep core.** The default UI must be approachable; advanced workflows live in command palette, inspector, graph, plugins, and AI tools.
2. **Typed everything.** Scene operations, components, assets, plugins, AI tools, and robotics messages have schemas.
3. **Inspectable AI.** AI actions must be previewable, replayable, revertible, and auditable.
4. **Large-scene first.** Scenes with billions of points or millions of objects must be streamable and partially loaded.
5. **Interoperability over lock-in.** USD, glTF, PLY, LAS/LAZ, E57, URDF, MJCF, ROS2, and open APIs are essential.
6. **Local-first, cloud-optional.** Desktop must work offline; cloud unlocks collaboration, remote GPU, asset indexing, and team workflows.
7. **Security by capability.** Plugins and agents must have explicit permissions.

---

## 2. Product and Data Model

### 2.1 Core nouns

| Concept | Meaning |
|---|---|
| Workspace | A user/team project space containing scenes, assets, plugins, comments, branches, and settings. |
| Project | A versioned unit on disk/cloud. May contain one or more scenes. |
| SceneDoc | Authoritative scene document: entities, components, layers, references, variants, metadata, and operation log. |
| Stage | A composed view of one or more SceneDocs, references, layers, variants, and live inputs. Similar in spirit to a USD stage, but internal and transactional. |
| Entity | Stable scene object identity. May represent mesh object, robot link, camera, splat set, point cloud tile, light, group, annotation, etc. |
| Component | Typed data attached to an entity. Transform, MeshRef, MaterialBinding, PointCloudRef, RobotJoint, SemanticTag, etc. |
| Asset | Content-addressed external or generated resource: mesh, texture, material, point cloud, splat, animation, robot model, AI output. |
| Operation | Typed mutation: create entity, set component, move, import asset, assign material, edit mesh, create keyframe, connect ROS topic. |
| Transaction | Ordered set of operations with author, timestamp, dependencies, validation result, undo data, and provenance. |
| Agent | Human, AI model, plugin, cloud worker, or robotics bridge that can read or mutate scenes through permissions. |
| Tool | Typed callable capability exposed to UI, agent, plugin, or CLI. |

### 2.2 Project format

Create3D should use a directory/pack model rather than only one binary file.

Recommended project layout:

```text
project.c3d/
  manifest.c3d.toml
  scenes/
    main.c3dscene
    variants/
  assets/
    blobs/
    index.c3dassetdb
  ops/
    log.c3dops
  thumbnails/
  ai/
    memory/
    generated/
  robotics/
    ros_profiles/
  plugins.lock
```

For distribution, the same structure can be packed into `.c3dpkg`.

### 2.3 Authoritative internal representation

The authoritative representation is **SceneDB**, not ECS and not renderer state.

SceneDB stores:

- entities with stable IDs,
- typed components with schema versions,
- parent/child and reference relationships,
- layers and variants,
- asset bindings,
- operation history,
- comments and annotations,
- semantic tags,
- live external data bindings,
- provenance from AI and importers.

ECS, renderer state, physics state, AI context, and robotics visualizations are derived projections.

### 2.4 Scene operation model

All scene mutations go through operations.

Operation categories:

| Category | Examples |
|---|---|
| Entity ops | create, delete, duplicate, group, instance, rename |
| Transform ops | translate, rotate, scale, set matrix, align, snap, parent |
| Component ops | add, remove, update field, migrate schema |
| Asset ops | import, reimport, bind, generate thumbnail, replace dependency |
| Geometry ops | extrude, bevel, boolean, decimate, retopo, convert, triangulate |
| Point cloud ops | crop, classify, downsample, align, segment, tile, colorize |
| Gaussian ops | import, optimize, crop, classify, convert, LOD build |
| Animation ops | create clip, set key, retarget, generate motion, edit curve |
| Procedural ops | create node, connect, evaluate, bake, expose parameter |
| Robotics ops | import URDF, set joint, bind ROS topic, set nav goal, record bag |
| Collaboration ops | comment, resolve, branch, merge, lock, permission update |
| AI ops | propose plan, execute tool, attach provenance, request approval |

Every operation must support:

- validation,
- undo/redo,
- serialization,
- optional preview,
- permission check,
- provenance,
- deterministic replay when possible.

### 2.5 Scene schema model

Create3D components should be schema-first.

Each component schema defines:

- stable type ID,
- version,
- fields,
- units,
- default values,
- migration functions,
- serialization format,
- editor UI hints,
- agent-access policy,
- network sync policy,
- runtime projection policy.

Examples:

| Component | Fields |
|---|---|
| Transform | local transform, world cache, parent, constraints |
| MeshRef | asset ID, submesh, topology mode, material slots |
| PointCloudRef | asset ID, tile filter, attribute mapping, LOD policy |
| GaussianSplatRef | asset ID, LOD policy, SH degree, opacity scale |
| MaterialBinding | material asset ID, overrides, slots |
| Camera | projection, focal length, sensor size, clipping |
| Light | type, intensity, color, shape, shadow policy |
| RobotLink | link name, inertial, visual/collision refs |
| RobotJoint | joint type, axis, limit, current state, ROS binding |
| SemanticTag | class, labels, confidence, source, ontology |
| LiveBinding | protocol, endpoint, topic/path, QoS, transform mapping |

---

## 3. Core Architecture

### 3.1 High-level architecture

```text
+--------------------------------------------------------------+
| Apps                                                         |
| desktop editor | web viewer | CLI | headless | cloud worker  |
+--------------------------------------------------------------+
| Editor Workbench                                             |
| UI shell | viewport | inspector | command palette | panels    |
+--------------------------------------------------------------+
| Interaction Systems                                          |
| tools | gizmos | selection | undo | comments | collaboration |
+--------------------------------------------------------------+
| AI / Procedural / Robotics / Plugin Layer                    |
| agents | tool registry | node graphs | ROS bridge | WASM host |
+--------------------------------------------------------------+
| Scene + Asset Core                                           |
| SceneDB | ops | ECS projection | AssetDB | schemas | importers |
+--------------------------------------------------------------+
| Geometry Data Engines                                        |
| mesh | B-Rep | NURBS | SDF | point cloud | Gaussian splat    |
+--------------------------------------------------------------+
| Runtime Engines                                              |
| animation | physics | simulation | task scheduler | time       |
+--------------------------------------------------------------+
| GPU Platform                                                 |
| RHI | render graph | compute graph | resource allocator      |
+--------------------------------------------------------------+
| Platform + Cloud                                             |
| storage | networking | auth | sync | remote GPU | telemetry    |
+--------------------------------------------------------------+
```

### 3.2 Architectural layers

| Layer | Responsibility | Must not depend on |
|---|---|---|
| Core | IDs, math, errors, logging, schema primitives, serialization traits | Editor, renderer, AI, robotics |
| Scene | SceneDB, entities, components, ops, layers, variants | Editor UI, specific renderer backend |
| Asset | content addressing, import/export, metadata, previews | Editor UI |
| Engine | app lifecycle, scheduler, ECS projection, plugins, time | Desktop-only UI |
| Geometry | mesh, B-Rep, NURBS, SDF, conversion, processing | Editor UI, cloud services |
| PointCloud | point storage, tiling, streaming, classification | Editor UI |
| Gsplat | Gaussian splat storage/render prep/editing | Editor UI |
| Renderer | RHI, render graph, pipelines, materials | AI provider, ROS2 implementation |
| Editor | UI, viewport, tools, command palette, panels | Cloud-only services |
| AI | tool protocol, planning, model routing, context builder | Renderer backend internals |
| Robotics | ROS2 bridge, URDF/MJCF import, live bindings | Editor UI implementation |
| Collaboration | sync, CRDT/OT, presence, comments, branches | Specific UI toolkit |
| SDK | public plugin/tool API | internal unstable modules |

### 3.3 Engine Core

Engine Core is the orchestration layer shared by editor, headless CLI, server workers, and tests.

Responsibilities:

- application lifecycle,
- project/session loading,
- async task scheduler,
- frame/update loop,
- event bus,
- command registry,
- tool registry,
- ECS world management,
- plugin loading,
- time and clock management,
- diagnostics and profiling,
- capability permissions,
- service locator for stable engine services.

Core services:

| Service | Responsibility |
|---|---|
| AppKernel | starts/stops services, loads project, owns global lifecycle |
| TaskGraph | CPU jobs, async jobs, blocking import jobs, GPU job dependencies |
| EventBus | typed events: selection changed, asset imported, ROS message, AI proposal |
| CommandBus | user/agent commands routed to tool handlers |
| TransactionManager | validates, applies, undoes, redoes, syncs scene transactions |
| SchemaRegistry | component schemas, migrations, serialization |
| PluginHost | native/WASM/Python plugin lifecycle and capability boundaries |
| Diagnostics | tracing, frame timing, memory, GPU timing, network stats |
| ClockService | editor time, simulation time, ROS time, playback time |

### 3.4 Scene System

The Scene System is the heart of Create3D.

Responsibilities:

- stable entity identity,
- typed components,
- transform graph,
- scene layers,
- composition/references,
- variants,
- operation log,
- undo/redo,
- scene diff/merge,
- collaboration sync,
- runtime projection into ECS/render/physics/AI contexts.

#### 3.4.1 SceneDB internals

Recommended internal storage:

- entity table: dense index + stable UUID/ULID,
- component column stores by type,
- chunked component storage for large homogeneous sets,
- transform DAG with dirty propagation,
- layer masks and variant masks,
- asset reference table,
- operation log segmented by branch,
- dependency graph for procedural and asset updates,
- semantic index for AI and search.

Important distinction:

- **Authoring SceneDB:** persistent, collaborative, versioned, schema-migrated.
- **Runtime ECS:** fast simulation/update projection for current active stage.
- **RenderWorld:** GPU-oriented extracted render state.
- **PhysicsWorld:** simulation-oriented derived state.
- **AI Scene Context:** compact semantic and spatial summary.

#### 3.4.2 Scene composition

Create3D should support:

- nested scenes,
- external references,
- overrides,
- variants,
- layers,
- live bindings,
- instancing,
- unloaded proxies.

Composition order:

1. base scene,
2. referenced scenes/assets,
3. layer stack,
4. variant selection,
5. local overrides,
6. live data bindings,
7. preview-only temporary transactions.

This allows AI agents and users to preview edits without committing them.

### 3.5 Entity Component System

ECS is required, but it should not be the file format.

Use ECS for:

- editor tools,
- selection state,
- viewport interaction,
- runtime systems,
- animation evaluation,
- physics sync,
- robotics live updates,
- render extraction,
- simulation loops.

Do not use ECS alone for:

- scene history,
- collaborative merge,
- persistent schema migration,
- binary asset storage,
- semantic search,
- large point cloud storage.

Recommended approach:

- Phase 1: Bevy ECS as implementation backend for runtime/editor systems.
- Wrap it behind `c3d_ecs` so Create3D components and schedules remain controlled.
- SceneDB emits ECS projection changes through a synchronization system.
- Long-term: keep the option to replace or augment ECS without changing SceneDB.

#### 3.5.1 ECS scheduling model

Schedules:

| Schedule | Examples |
|---|---|
| Startup | load project, register schemas, initialize renderer |
| PreUpdate | process inputs, receive network/ROS/AI events |
| SceneApply | apply committed transactions to SceneDB |
| Projection | sync SceneDB changes to ECS worlds |
| Simulation | physics, robotics, animation, procedural time updates |
| RenderExtract | extract renderable data to RenderWorld |
| Render | build and submit GPU graph |
| PostUpdate | diagnostics, sync, autosave |

### 3.6 Asset System

The Asset System manages all external and generated content.

Responsibilities:

- content addressing,
- import/export,
- metadata extraction,
- previews/thumbnails,
- dependency tracking,
- asset variants,
- asset provenance,
- background conversion,
- cloud/local caching,
- license metadata,
- generated asset lineage.

Asset ID model:

- logical asset ID: stable within project/workspace,
- content hash: immutable blob identity,
- version ID: asset revision,
- import fingerprint: source file path, timestamp, importer version, options,
- provenance ID: human/importer/AI/plugin source.

Asset types:

| Type | Examples |
|---|---|
| Geometry | glTF/GLB, OBJ, FBX via optional plugin, STL, USD mesh |
| CAD | STEP/IGES via optional B-Rep plugin, USD, native B-Rep |
| Point cloud | LAS, LAZ, COPC, PLY, E57 |
| Gaussian | PLY-based 3DGS, native C3D splat pack |
| Materials | PBR materials, textures, procedural graphs |
| Textures | PNG, JPEG, EXR, HDR, KTX2/Basis |
| Animation | glTF clips, BVH, USD animation |
| Robotics | URDF, Xacro-expanded URDF, MJCF, robot meshes |
| AI outputs | generated mesh, material, texture, motion, scene plan |

#### 3.6.1 Import pipeline

Import pipeline stages:

1. detect file type,
2. create import task,
3. parse source,
4. normalize units and coordinates,
5. extract metadata,
6. generate internal asset representation,
7. generate preview and LODs,
8. validate,
9. store immutable blobs,
10. create asset version,
11. emit scene operation if user imports into scene.

Importers must be sandboxable because scene files and asset files can be untrusted.

### 3.7 Rendering System

The Rendering System is split into:

- RHI layer,
- GPU resource allocator,
- render graph,
- shader library,
- material compiler,
- scene extraction,
- raster renderer,
- path tracer,
- point cloud renderer,
- Gaussian splat renderer,
- editor viewport overlays,
- selection/id rendering,
- thumbnail/preview renderer.

Renderer must support both:

- interactive editor viewport,
- headless server rendering and asset processing.

### 3.8 Material System

The material system must serve rasterization, path tracing, point clouds, splats, robotics visualizations, and generated assets.

Architecture:

- Material Asset: graph + parameters + texture bindings + metadata.
- Material Instance: overrides per entity or selection.
- Material Compiler: graph to shader IR.
- Shader Backend: WGSL first; HLSL/SPIR-V/MSL via translation/backends where possible.
- Preview Builder: thumbnails and material ball.
- Agent Interface: natural language material edits map to parameter or graph operations.

Material graph node categories:

| Category | Examples |
|---|---|
| Input | UV, world position, normal, vertex color, point attribute |
| Texture | image, triplanar, procedural noise |
| BSDF | principled PBR, glass, emission, subsurface later |
| Utility | math, mix, remap, color correction |
| Procedural | noise, voronoi, curvature, ambient occlusion |
| AI | generated texture slot, prompt/provenance metadata |
| Robotics | semantic color, costmap color, sensor confidence |
| Point/Splat | intensity, classification, SH color, opacity scaling |

### 3.9 Geometry Kernel

The Geometry Kernel is a multi-representation geometry layer.

Supported representations:

- Mesh,
- B-Rep,
- NURBS,
- SDF,
- Point Cloud,
- Gaussian Splat.

Core abstraction:

- `GeometryAsset`: persistent asset-level object,
- `GeometryView`: filtered/LOD/subset view,
- `GeometryHandle`: stable reference from scene component,
- `GeometryOp`: typed geometry operation,
- `GeometryCache`: derived artifacts: render buffers, collision meshes, previews, BVH.

Design goals:

- support mixed geometry in one scene,
- allow conversion between representations,
- separate authoring topology from render buffers,
- support huge assets by tiling/chunking,
- attach semantic labels and provenance to geometry elements,
- make geometry operations callable by UI, procedural nodes, and AI tools.

### 3.10 Mesh Processing

Mesh Processing includes:

- topology editing,
- triangulation,
- normal/tangent generation,
- UV unwrap hooks,
- simplification/decimation,
- remeshing,
- boolean operations,
- bevel/extrude/inset,
- mesh repair,
- meshlet/cluster generation,
- collision mesh generation,
- retopology hooks,
- point cloud reconstruction.

Implementation strategy:

- Phase 1: simple indexed mesh + basic edit ops + glTF import/export.
- Phase 2: half-edge authoring mesh and mesh repair/decimation.
- Phase 3: robust boolean/remesh, SDF conversion, GPU mesh processing.
- Phase 4: optional B-Rep/CAD kernel integration.

### 3.11 Point Cloud Engine

Point Cloud Engine must be designed for massive datasets.

Responsibilities:

- LAS/LAZ/COPC/PLY/E57 import,
- coordinate reference metadata,
- unit normalization,
- chunked octree storage,
- streaming LOD,
- GPU upload cache,
- attribute filtering,
- classification,
- crop/clip tools,
- registration/alignment,
- segmentation,
- point picking,
- point-to-mesh/SDF/Gaussian conversions.

Internal storage:

- root metadata: bounds, CRS, units, point count, attributes,
- chunk tree: octree or hierarchical grid,
- chunk payloads: compressed point blocks,
- attribute SoA: position, color, intensity, normal, classification, timestamp, semantic label,
- LOD summaries: representative points per chunk,
- GPU cache: resident chunk buffers with LRU eviction.

### 3.12 Gaussian Splatting Engine

Gaussian Splatting is not a plugin afterthought. It is a native geometry/rendering representation.

Responsibilities:

- import 3D Gaussian splat datasets,
- render splats in viewport,
- crop/select/edit splats,
- LOD and streaming,
- convert point cloud to splat pipeline hooks,
- convert splat to mesh or proxy collision where possible,
- material/semantic overlays,
- train/optimize through external ML pipeline adapters.

Internal splat representation:

- means: 3D positions,
- covariance representation: rotation + scale, or packed covariance,
- opacity,
- color: SH coefficients or RGB fallback,
- semantic label/confidence,
- source image/camera metadata,
- quality metrics,
- chunk/LOD membership.

Render strategy:

- project splats to screen,
- estimate screen-space ellipse,
- tile/bin splats,
- sort or approximate-sort per tile,
- alpha composite front-to-back/back-to-front depending pipeline,
- support classification and selection overlays.

### 3.13 Animation System

The Animation System must support artist animation, generated animation, robotics joint animation, camera animation, and simulation playback.

Core components:

- timeline,
- clips,
- tracks,
- keyframes,
- curves,
- skeletal rigs,
- constraints,
- retargeting,
- animation layers,
- motion provenance,
- simulation recording,
- ROS bag/live replay adapter.

Data model:

- AnimationAsset: reusable clip or motion graph.
- Timeline: scene-level sequencing.
- Track: target entity/component field.
- Key: time/value/interpolation.
- Constraint: relation evaluated during animation/simulation.
- RetargetMap: source rig to target rig.

AI animation generation should produce editable tracks and curves, not only baked black-box animation.

### 3.14 Physics System

Physics has two separate roles:

1. **DCC physics:** collision previews, rigid bodies, simple constraints, layout validation.
2. **Robotics simulation bridge:** physical properties, joints, sensors, simulation time, external simulator integration.

Recommended strategy:

- Phase 1: use a Rust-native physics/collision stack for editor selection, colliders, simple rigid bodies, and robot link collisions.
- Phase 2: simulation adapter interface.
- Phase 3: MuJoCo/Gazebo/Isaac/other external simulator bridges through plugins.
- Phase 4: deterministic simulation recording and digital twin playback.

Physics data must be derived from scene components and assets, not manually duplicated.

### 3.15 Procedural System

The Procedural System should be a practical, typed, incremental graph system.

Goals:

- allow artists to build procedural assets and scenes,
- allow AI agents to create and modify procedural graphs,
- support deterministic evaluation,
- support partial recomputation,
- support caching/baking,
- work across mesh, point cloud, SDF, splat, material, animation, and robotics domains.

Node graph properties:

- typed inputs/outputs,
- versioned node definitions,
- deterministic seeds,
- asset dependencies,
- dirty propagation,
- cache keys,
- evaluation budget,
- preview output,
- bake-to-scene transaction.

Procedural graph categories:

| Domain | Examples |
|---|---|
| Geometry | scatter, extrude, boolean, bevel, remesh, convert |
| Scene | place assets, align, distribute, layout, instantiate |
| Point cloud | crop, classify, downsample, segment, reconstruct |
| Gaussian | crop, LOD, classify, convert, optimize hook |
| Material | procedural texture, semantic material assignment |
| Animation | camera path, motion curve, procedural loop |
| Robotics | generate collision proxy, sensor frustum, nav test setup |
| AI | prompt node, asset search node, semantic segmentation node |

### 3.16 AI Agent Framework

AI Agent Framework is Create3D’s largest differentiator.

It must provide:

- scene-aware context building,
- tool capability registry,
- planner/executor separation,
- transaction generation,
- validation,
- preview,
- approval flow,
- provenance,
- memory/RAG,
- model provider abstraction,
- sandboxed code/tool execution,
- agent permissions.

Agent pipeline:

```text
User intent
→ Context Pack Builder
→ Intent Parser
→ Planner
→ Tool Plan
→ Permission Check
→ Tool Execution
→ Scene Transaction Preview
→ Validators
→ Human/Policy Approval
→ Commit
→ Sync/Provenance/Undo
```

All agents must use the same tool protocol as UI commands and plugins.

### 3.17 Plugin Framework

Rust does not have a stable native ABI suitable for arbitrary third-party binary plugins. Therefore Create3D should separate internal crates from public plugin ABI.

Plugin modes:

| Mode | Use | Pros | Cons |
|---|---|---|---|
| Internal Rust crate | core repo modules | fastest, type-safe | not stable ABI |
| Native C ABI plugin | high-performance external integrations | stable boundary | unsafe, platform packaging |
| WASM plugin | sandboxed tools, importers, procedural nodes | secure, portable | performance limits, host API design needed |
| Python script plugin | automation, AI glue, user scripting | easy ecosystem | sandboxing and performance concerns |
| Sidecar service | AI models, ROS2, heavy importers | isolated, language-agnostic | IPC complexity |

Recommended policy:

- public plugins use WASM or sidecar by default,
- native plugins require explicit trust,
- Python is opt-in and sandboxed where possible,
- core built-in modules are Rust workspace crates,
- all plugins expose typed manifests and capability requirements.

Plugin manifest should define:

- plugin ID,
- version,
- capabilities,
- commands,
- tools,
- component schemas,
- import/export handlers,
- UI panels,
- node definitions,
- permissions,
- minimum Create3D version.

### 3.18 Collaboration Framework

Collaboration is not just “multiple users editing one file.” It requires document modeling, sync, permissions, presence, comments, branch/merge, and conflict semantics.

Recommended model:

- local-first SceneDB with operation log,
- sync server for team workspaces,
- CRDT/OT-inspired field-level merge where safe,
- server-authoritative conflict resolution for unsafe geometry operations,
- pessimistic locks for destructive mesh edits and heavy imports,
- optimistic transactions for transforms/materials/annotations,
- branch and merge for major scene changes,
- review/approval for AI-generated transactions.

Collaboration features:

| Feature | Description |
|---|---|
| Presence | active users, cursor/ray, selected objects |
| Comments | anchored to entity, point, surface, frame, time range |
| Branches | experiment with scene variants and AI proposals |
| Scene diff | entity/component/asset/geometry-level summaries |
| Merge | operation-log merge with validators |
| Locks | optional object/asset/geometry edit locks |
| Permissions | workspace, scene, asset, plugin, agent capabilities |
| Offline editing | local operation log syncs later |

---

## 4. Rendering Architecture

### 4.1 Rendering API comparison

| API | Strengths | Weaknesses | Create3D position |
|---|---|---|---|
| Vulkan | Cross-platform, low-level, explicit, modern GPU control, strong for Linux/Windows/Android and pro tooling | Complex; Apple support requires portability layer or separate Metal path | Strategic native backend for high-end GPU-native renderer, ray tracing, bindless, advanced compute |
| Metal | Best Apple platform integration, low-overhead, strong Apple Silicon support | Apple-only; different shader/tooling ecosystem | Required for macOS/iPadOS excellence through wgpu initially, possible native advanced backend later |
| DirectX 12 | Best Windows/Xbox integration, modern explicit GPU model, DXR ecosystem | Windows-centric | Supported through wgpu initially; native backend later only if needed |
| OpenGL | Mature, broad legacy support, easier conceptual model | Driver overhead, weak modern explicit control, not ideal for GPU-native architecture | Compatibility fallback only; not a strategic renderer foundation |

### 4.2 Recommended rendering stack

Create3D should use a two-layer strategy:

1. **Phase 1 portability layer:** `wgpu` backend through an internal RHI facade.  
   This enables Vulkan, Metal, Direct3D 12, OpenGL/WebGL/WebGPU targets without building every backend immediately.

2. **Phase 3+ advanced native backend:** Vulkan-first native backend for advanced GPU-driven rendering, ray tracing, path tracing, mesh/task-style pipelines where available, and specialized compute pipelines.  
   Metal and DirectX 12 native paths can be introduced only when wgpu or portability layers block product goals.

The Create3D renderer code should never directly depend on wgpu APIs outside `c3d_rhi_wgpu`. The engine depends on `c3d_rhi` traits and resource abstractions.

### 4.3 RHI design

RHI modules:

| Module | Responsibility |
|---|---|
| Adapter | enumerate GPUs, features, limits, memory budgets |
| Device | create resources, pipelines, queues |
| Queue | graphics, compute, copy submission |
| ResourceAllocator | buffers, textures, heaps, transient resources |
| BindLayout | material/resource binding abstraction |
| PipelineCache | graphics/compute/ray pipeline variants |
| ShaderCompiler | WGSL/HLSL/SPIR-V/MSL translation path abstraction |
| CommandEncoder | command recording abstraction |
| RenderGraphBridge | converts render graph passes into RHI work |
| TimestampProfiler | GPU timing |
| DebugLayer | labels, captures, validation hooks |

RHI must expose feature flags:

- storage buffers,
- indirect draw,
- multi-draw indirect,
- compute shaders,
- subgroup/wave operations,
- bindless/resource arrays,
- ray tracing acceleration structures,
- timeline semaphores/fences,
- mesh/task shader equivalents where available,
- 16-bit/8-bit shader arithmetic,
- sparse/virtual textures if supported.

### 4.4 RenderWorld extraction

SceneDB cannot be traversed directly by the renderer each frame. Renderer uses RenderWorld.

Extraction pipeline:

1. SceneDB transactions mark changed entities/assets.
2. ECS projection updates runtime state.
3. Render extraction builds/updates RenderWorld objects.
4. RenderWorld stores GPU-ready handles and visibility metadata.
5. RenderGraph consumes RenderWorld to build passes.

RenderWorld contains:

- renderable instances,
- material instances,
- meshlet/cluster buffers,
- point cloud chunk handles,
- splat chunk handles,
- lights,
- cameras,
- selection IDs,
- debug overlays,
- acceleration structure metadata,
- streaming residency state.

### 4.5 RenderGraph

RenderGraph should be explicit and data-driven.

Pass categories:

| Category | Examples |
|---|---|
| Scene prep | culling, LOD selection, cluster compaction, splat binning |
| G-buffer | depth, normals, material IDs, motion vectors |
| Forward | transparent, splats, overlays, special materials |
| Lighting | deferred lighting, shadows, IBL, probes |
| Ray tracing | TLAS/BLAS update, rays, denoising |
| Path tracing | sample accumulation, denoise, display |
| Point cloud | point splat/EDL, classification colors |
| Gaussian | tile sort, compositing |
| Editor | grid, axes, gizmos, outlines, selection IDs |
| Post | tone mapping, TAA, upscaling, color management |
| UI | 2D UI composition |
| Capture | screenshots, thumbnails, render exports |

### 4.6 Rasterization pipeline

Target: high-performance interactive viewport for massive scenes.

Pipeline:

1. camera and frustum setup,
2. scene chunk selection,
3. CPU broadphase for unloaded/low-cost objects,
4. GPU culling for instances/clusters,
5. LOD selection,
6. indirect draw buffer generation,
7. depth prepass or visibility pass,
8. material/G-buffer pass,
9. lighting/shadow passes,
10. transparent/splat/point passes,
11. editor overlays,
12. post processing.

Key features:

- GPU-driven culling,
- meshlet/cluster render assets,
- instancing,
- virtualized geometry cache,
- bindless-like material/texture indirection when backend supports it,
- selection ID rendering,
- editor overlays rendered in separate compositable passes,
- scalable debug rendering for robotics and point clouds.

### 4.7 Ray tracing pipeline

Ray tracing should not block the MVP, but architecture must allow it.

Responsibilities:

- BLAS build/update for meshes,
- TLAS build/update for instances,
- material hit group mapping,
- ray-gen shader abstraction,
- denoiser integration,
- viewport preview mode,
- path tracer integration,
- robotics sensor simulation hooks later.

Use cases:

- high-quality previews,
- shadows/reflections/GI,
- path tracing,
- lightmap/probe baking,
- synthetic camera/LiDAR simulation later.

### 4.8 Path tracing pipeline

Path tracer is a progressive offline/interactive-quality renderer.

Architecture:

- independent render mode using same SceneDB/Material assets,
- sample accumulation buffer,
- adaptive sampling later,
- denoising,
- spectral/advanced materials later,
- headless rendering support,
- cloud GPU rendering support.

Path tracing must not compromise viewport interactivity. It is a render mode, not the only renderer.

### 4.9 Neural rendering pipeline

Neural rendering should be an extensible subsystem, not one hard-coded technique.

Supported categories:

- learned denoising,
- neural materials/textures,
- NeRF-style view synthesis via plugin,
- image-to-3D/text-to-3D asset generation,
- neural reconstruction from images/video,
- semantic segmentation of point clouds/images,
- upscaling/frame reconstruction if appropriate.

Architecture:

- model provider abstraction,
- GPU/CPU tensor bridge,
- external model server sidecar,
- asset provenance and reproducibility metadata,
- generated asset validation,
- non-blocking job scheduling.

### 4.10 Gaussian rendering pipeline

Gaussian rendering requires a separate pipeline from mesh rasterization.

Pipeline:

1. visible splat chunk selection,
2. GPU upload residency check,
3. project means to screen,
4. compute covariance in screen space,
5. cull tiny/invisible splats,
6. tile binning,
7. depth sort per tile or approximate order,
8. alpha compositing,
9. overlay selection/classification,
10. composite with mesh/point cloud passes.

Editor-specific requirements:

- select splats by brush/lasso/frustum,
- crop and hide selected splats,
- paint semantic labels,
- visualize source camera confidence,
- stream large splat scenes,
- convert selected splats to point cloud or mesh proxy.

---

## 5. Geometry Architecture

### 5.1 Geometry principles

Create3D must not be mesh-only. The internal geometry architecture must allow multiple forms to coexist.

Principles:

1. **Representation-specific storage.** Mesh, B-Rep, SDF, point cloud, and Gaussian splat should use data structures suited to their workloads.
2. **Unified scene binding.** Scene entities reference geometry assets uniformly through component handles.
3. **Conversion graph.** Geometry conversion is explicit, cached, and provenance-tracked.
4. **Stable element IDs.** Vertices, faces, edges, points, splats, surfaces, and joints should have stable IDs where editing and annotations require it.
5. **Attribute-first design.** Geometry elements carry typed attributes: UVs, normals, colors, semantics, confidence, source, timestamp, material slots.
6. **Tiled/chunked by default.** Large geometry should be partially loaded and GPU-resident on demand.
7. **Units and coordinate systems are explicit.** Essential for CAD, robotics, GIS, point clouds, and digital twins.

### 5.2 Geometry asset hierarchy

```text
GeometryAsset
  MeshAsset
    AuthoringMesh
    RenderMeshCache
    CollisionMeshCache
  BrepAsset
    Topology
    ParametricGeometry
    TessellationCache
  NurbsAsset
    Curves
    Surfaces
    TrimData
  SdfAsset
    FieldGraph
    SparseVolumeCache
  PointCloudAsset
    Octree
    Chunks
    Attributes
  GaussianSplatAsset
    SplatChunks
    SHData
    Acceleration/LOD
```

### 5.3 Mesh architecture

#### 5.3.1 Mesh representations

Create3D should maintain separate mesh forms:

| Representation | Purpose |
|---|---|
| Authoring mesh | editing topology, stable IDs, non-triangulated faces |
| Render mesh | GPU-ready triangles/meshlets, packed attributes |
| Collision mesh | simplified physics/collision representation |
| Analysis mesh | adjacency, curvature, geodesic, validation data |
| Preview proxy | low LOD for streaming and thumbnails |

#### 5.3.2 Authoring mesh: Chunked Half-Edge Mesh

Recommended internal design:

- vertices table,
- half-edge table,
- edge table,
- face table,
- loop/corner table,
- attribute layers,
- stable element IDs,
- chunking for partial edits,
- dirty ranges for incremental render/cache rebuild.

Core tables:

| Table | Fields |
|---|---|
| Vertex | stable ID, position, outgoing half-edge, flags, attributes |
| HalfEdge | vertex, twin, next, prev, face, edge, corner attrs |
| Edge | stable ID, half-edge, crease/sharp flags, attributes |
| Face | stable ID, first half-edge, material slot, normal cache, attributes |
| Loop/Corner | face-vertex attributes: UV, normal, color, tangent |
| AttributeLayer | name, type, domain, storage, interpolation |

Attribute domains:

- vertex,
- edge,
- face,
- corner,
- instance,
- material slot,
- semantic region.

#### 5.3.3 Render mesh

Render mesh should be GPU-first:

- packed vertex buffer,
- index buffer,
- meshlet/cluster buffer,
- bounding volumes per cluster,
- material ranges,
- LODs,
- quantized formats where possible,
- optional compressed geometry.

Render mesh rebuilds from authoring mesh through a cache job.

### 5.4 B-Rep architecture

B-Rep is required for CAD-like workflows and robotics/digital twin interoperability, but robust B-Rep kernels are complex. Create3D should define a clean internal API and initially support B-Rep import/view/tessellation via optional backend.

#### 5.4.1 B-Rep topology

Topology hierarchy:

```text
Body
  Shell
    Face
      Loop
        CoEdge
          Edge
            Vertex
```

Tables:

| Element | Data |
|---|---|
| Body | ID, shells, metadata, units |
| Shell | orientation, faces |
| Face | surface ref, trim loops, orientation, tolerance |
| Loop | coedges, type outer/inner |
| CoEdge | edge ref, curve-on-surface ref, orientation |
| Edge | 3D curve ref, vertices, tolerance |
| Vertex | point, tolerance |

#### 5.4.2 B-Rep geometry

Geometry primitives:

- plane,
- cylinder,
- cone,
- sphere,
- torus,
- NURBS surface,
- analytic curves,
- NURBS curves,
- trimmed surfaces.

B-Rep requirements:

- tolerance tracking,
- robust tessellation,
- unit conversion,
- face/edge stable IDs,
- CAD metadata,
- feature recognition later,
- conversion to render mesh/collision mesh/SDF.

Recommended implementation path:

- Phase 1: no native B-Rep editing; support schema and placeholders.
- Phase 2: import/export through optional C++ kernel plugin.
- Phase 3: tessellation, selection, measurements, metadata.
- Phase 4: limited direct modeling operations.

### 5.5 NURBS architecture

NURBS supports curves and surfaces both as standalone geometry and as B-Rep geometry.

NURBS curve data:

- degree,
- knot vector,
- control points,
- weights,
- domain,
- periodic flag,
- attributes.

NURBS surface data:

- degree U/V,
- knot vectors U/V,
- control grid,
- weights,
- domain U/V,
- periodic flags,
- trim loops if used as surface asset,
- tessellation cache.

Use cases:

- CAD import,
- curves/splines,
- camera paths,
- robotics trajectories,
- procedural profiles,
- animation curves.

### 5.6 SDF architecture

SDF is critical for booleans, procedural modeling, collision proxies, robotics volumes, and conversion workflows.

Representations:

| Type | Use |
|---|---|
| Analytic SDF node | primitives, booleans, procedural fields |
| Dense volume | small fields, GPU processing |
| Sparse brick volume | large fields and narrow bands |
| Signed distance cache | mesh conversion, collision, remeshing |
| Neural/learned field plugin | future neural implicit workflows |

SDF FieldGraph nodes:

- primitive sphere/box/cylinder/capsule,
- transform,
- union/intersection/difference,
- smooth union,
- displacement/noise,
- mesh-to-SDF sample,
- point-cloud-to-SDF reconstruction,
- offset/dilate/erode,
- material/semantic field.

SDF storage:

- brick grid with fixed brick resolution,
- root bounds,
- voxel size,
- narrow-band distance range,
- compression,
- GPU brick pool,
- dirty brick updates.

### 5.7 Point Cloud architecture

#### 5.7.1 PointCloudAsset

PointCloudAsset metadata:

- point count,
- bounding box,
- coordinate reference system,
- unit scale,
- source file metadata,
- acquisition metadata,
- attributes,
- classification schema,
- chunk hierarchy,
- LOD levels,
- compression format,
- provenance.

#### 5.7.2 Point chunk data

Use structure-of-arrays for high throughput:

| Attribute | Storage |
|---|---|
| Position | quantized local coordinates + chunk origin/scale |
| Color | RGB/RGBA 8/16-bit |
| Intensity | 16-bit or float |
| Normal | packed normalized vector |
| Classification | integer class |
| Semantic label | integer ontology ID |
| Timestamp | optional |
| Confidence | optional float/half |
| Source ID | scanner/camera/frame ID |

Chunk hierarchy:

- root octree/grid,
- chunk bounds,
- point count,
- LOD representatives,
- compressed payload offset,
- GPU residency state,
- dirty flags for edits.

### 5.8 Gaussian Splat architecture

#### 5.8.1 GaussianSplatAsset

Gaussian splat metadata:

- splat count,
- bounds,
- SH degree,
- training metadata,
- source cameras,
- quality metrics,
- coordinate system,
- chunk hierarchy,
- LODs,
- semantic layers,
- provenance.

#### 5.8.2 Splat SoA storage

| Attribute | Storage |
|---|---|
| Mean | local quantized or float3 position |
| Rotation | quaternion packed 16-bit or float4 |
| Scale | log-scale packed half/float3 |
| Opacity | half/float |
| SH coefficients | packed half/float array, degree-dependent |
| RGB fallback | optional packed color |
| Semantic label | optional uint |
| Confidence | optional half/float |
| Source range | optional camera/image IDs |

Acceleration:

- chunked spatial hierarchy,
- screen-space tile binning,
- LOD per chunk,
- optional coarse proxy mesh/bounds,
- edit selection masks.

### 5.9 Geometry conversion graph

Supported conversions:

| From | To | Purpose |
|---|---|---|
| B-Rep/NURBS | Mesh | rendering, export, collision |
| Mesh | SDF | booleans, collision, remeshing |
| SDF | Mesh | procedural modeling, export |
| Point Cloud | Mesh | reconstruction |
| Point Cloud | SDF | occupancy/collision/reconstruction |
| Point Cloud | Gaussian | radiance/splat representation |
| Gaussian | Point Cloud | analysis, segmentation, editing |
| Gaussian | Mesh proxy | collision or approximate export |
| Mesh | Point Cloud | sampling/synthetic sensor data |
| Scene | USD/glTF | interoperability |

All conversions must store:

- source asset/version,
- algorithm,
- parameters,
- software version,
- timestamp,
- generated asset ID,
- quality metrics.

### 5.10 Units, coordinates, and precision

Create3D must support:

- meter-first internal units,
- explicit unit metadata per asset,
- coordinate transforms per import,
- local origin shifting for huge scenes,
- double precision for authoring/world transforms,
- float precision for GPU-local rendering,
- CRS metadata for geospatial point clouds,
- robotics frame IDs and TF mapping.

Large scenes require camera-relative rendering and chunk-local coordinates to avoid precision issues.

---

## 6. AI Native Architecture

### 6.1 AI architecture thesis

Create3D’s AI layer must not be a chat panel glued onto a DCC. It must be a structured agent runtime where AI can safely inspect, plan, modify, validate, and explain scene changes.

Core rule:

> AI never mutates hidden state directly. AI proposes or executes typed tools that produce scene transactions.

### 6.2 AI system layers

```text
AI UX
  chat | inline command | context menu | voice later | batch jobs
AI Orchestration
  context builder | model router | planner | executor | validator
Tool Protocol
  scene query | scene ops | geometry ops | asset ops | render ops | robotics ops
Scene Transaction Layer
  preview | diff | approval | commit | undo | provenance
Memory / Retrieval
  scene semantic index | asset index | docs | user/team preferences | examples
Model Providers
  local LLM | cloud LLM | text-to-3D | image-to-3D | segmentation | motion models
Sandbox
  permissions | resource limits | network limits | file limits | audit log
```

### 6.3 AI context builder

The context builder creates compact, relevant context for agents.

Inputs:

- user prompt,
- selected entities,
- viewport camera,
- scene hierarchy,
- semantic tags,
- asset metadata,
- current tool/mode,
- operation history,
- comments/tasks,
- robotics/live data state,
- user/team preferences,
- plugin/tool manifest.

Outputs:

- scene summary,
- selection summary,
- available tools,
- constraints,
- relevant assets,
- relevant docs,
- prior decisions,
- safety/permission context.

For huge scenes, AI must not receive raw scene dumps. It receives hierarchical summaries, semantic search results, spatial subsets, and tool outputs.

### 6.4 AI tool protocol

Every AI-callable tool must define:

- name,
- description,
- input schema,
- output schema,
- side effects,
- required permissions,
- cost estimate,
- preview support,
- validation hooks,
- undo strategy,
- timeout/resource limits,
- provenance output.

Tool categories:

| Category | Example tools |
|---|---|
| Scene query | list entities, search semantic tags, inspect selection, find collisions |
| Scene edit | create entity, transform, group, replace, align, distribute |
| Geometry | create primitive, extrude, bevel, boolean, remesh, decimate, crop point cloud |
| Material | create material, assign, generate texture, edit parameter |
| Asset | search/import/generate/replace asset, check license |
| Render | create preview, compare before/after, generate thumbnail |
| Animation | create timeline, set keyframes, generate camera path, retarget motion |
| Robotics | import URDF, bind topic, set joint, visualize costmap, run validation |
| Collaboration | comment, propose branch, summarize diff |
| Procedural | create graph, add node, expose parameter, bake output |

### 6.5 Agent types

#### 6.5.1 AI Copilot

Role: general assistant inside editor.

Responsibilities:

- answer scene questions,
- explain tools,
- suggest next steps,
- generate command plans,
- summarize diffs,
- create comments/tasks,
- route specialized work to other agents.

Allowed actions by default:

- read scene summaries,
- inspect selected objects,
- propose operations,
- run preview tools,
- require user approval before commit.

#### 6.5.2 Scene Agent

Role: high-level scene organization and editing.

Responsibilities:

- layout objects,
- arrange scenes,
- enforce naming/layer conventions,
- optimize hierarchy,
- place cameras/lights,
- manage variants,
- create scene diffs.

Tools:

- transform, align, distribute,
- group/ungroup,
- create collection/layer,
- assign semantic labels,
- validate scene scale and units,
- run collision/clearance checks.

#### 6.5.3 Modeling Agent

Role: geometry creation and modification.

Responsibilities:

- generate primitives and procedural shapes,
- modify mesh topology,
- create simple assets from descriptions,
- convert point clouds to usable geometry,
- retopology and repair,
- prepare collision proxies.

Outputs:

- mesh assets,
- procedural graphs,
- SDF graphs,
- generated asset provenance,
- editable scene operations.

#### 6.5.4 Animation Agent

Role: animation and camera/motion generation.

Responsibilities:

- create camera moves,
- generate object animations,
- generate robot joint playback,
- retarget motion,
- create timeline beats from text,
- smooth curves,
- export preview.

Outputs:

- animation tracks,
- keyframes,
- constraints,
- timeline markers,
- render previews.

#### 6.5.5 Asset Agent

Role: asset discovery, import, cleanup, and generation.

Responsibilities:

- search local/team asset libraries,
- import external formats,
- generate thumbnails,
- deduplicate assets,
- recommend replacements,
- check license/provenance,
- create LODs and variants.

#### 6.5.6 Robotics Agent

Role: robotics scene setup, debugging, and validation.

Responsibilities:

- import URDF/MJCF,
- map frames to scene transforms,
- bind ROS2 topics/services/actions,
- visualize TF, joint states, point clouds, costmaps,
- set Nav2 goals,
- inspect Autoware planning outputs,
- validate collision and joint limits,
- generate test scenarios.

#### 6.5.7 World Building Agent

Role: generate and manage larger environments.

Responsibilities:

- create environment layouts,
- place assets semantically,
- produce terrain/building/interior variations,
- synthesize robotics test worlds,
- create digital twin layers from scans,
- enforce style and scale constraints.

### 6.6 Natural language to model generation

Pipeline:

1. Parse intent: object type, style, dimensions, constraints, target use.
2. Select generation route:
   - procedural primitive/graph,
   - mesh generation model,
   - kitbash from asset library,
   - SDF/CAD-like construction,
   - point-cloud reconstruction.
3. Generate candidate(s).
4. Validate:
   - scale,
   - topology quality,
   - manifoldness if required,
   - material completeness,
   - collision proxy,
   - license/provenance.
5. Create asset version.
6. Place in scene through transaction.
7. Attach provenance and prompt.
8. Offer editable parameters or procedural graph when possible.

Important: generated models must not become uneditable black boxes. Prefer procedural graphs, semantic parts, material slots, and clean transforms.

### 6.7 Natural language to scene editing

Pipeline:

1. Resolve references: “this”, “the robot”, “the warehouse wall”, “all red pipes”.
2. Query scene context.
3. Build candidate operation plan.
4. Preview changes as temporary transaction.
5. Validate constraints:
   - collisions,
   - scale,
   - parenting,
   - locked objects,
   - robotics limits,
   - layer permissions.
6. Ask for approval if destructive or high impact.
7. Commit transaction.
8. Summarize diff.

Example operation types:

- “Move all pallets 1m away from the robot path.”
- “Replace these boxes with low-poly proxies.”
- “Make this room navigable for a differential-drive robot.”
- “Turn this scan into semantic layers: floor, wall, ceiling, machines.”

### 6.8 Natural language to animation generation

Pipeline:

1. Parse narrative or task.
2. Identify actors and timeline duration.
3. Generate motion/camera plan.
4. Create animation tracks and constraints.
5. Validate collisions and timing.
6. Preview in viewport.
7. Commit editable curves/keyframes.

Animation outputs should always be editable with standard timeline tools.

### 6.9 AI safety and trust

AI systems must include:

- permissioned tool access,
- destructive-operation approval,
- preview before commit,
- operation diff,
- provenance metadata,
- generated asset license/source tracking,
- model/provider logs where allowed,
- sandboxing for code execution,
- resource budgets,
- offline/local model mode for sensitive scenes,
- workspace-level AI policy.

---

## 7. Robotics and Digital Twin Architecture

### 7.1 Robotics goals

Create3D should be a serious robotics scene workbench, not only a pretty viewer.

Goals:

- import and edit robot models,
- visualize live ROS2 systems,
- build simulation-ready environments,
- process point clouds and maps,
- support Nav2 and Autoware workflows,
- validate clearances, collisions, frames, and sensor placement,
- record/replay live robotics sessions,
- generate synthetic scenarios with AI.

### 7.2 Supported ecosystems

Required first-class integrations:

| Ecosystem | Create3D support |
|---|---|
| ROS2 | topics, services, actions, parameters, TF, QoS profiles, rosbag later |
| URDF | robot import, links, joints, inertials, visuals, collisions, materials |
| MJCF | MuJoCo model import/export adapter, bodies, joints, actuators, geoms |
| USD | scene interchange, digital twin scenes, references, animation, variants |
| Autoware | maps, point clouds, perception/planning/control visualization, diagnostics |
| Nav2 | costmaps, robot pose, path, goals, behavior/status visualization |

### 7.3 Robotics architecture

```text
Robotics UI Panels
  robot tree | TF viewer | topic browser | nav goal | diagnostics
Robotics Agent
  setup | debug | validate | scenario generation
Robotics Core
  robot model | kinematics | frames | sensors | live bindings
ROS2 Bridge
  topics | services | actions | parameters | QoS | TF
Simulation Adapters
  internal physics | MuJoCo | Gazebo | Isaac | external process
Scene Integration
  SceneDB components | assets | point clouds | animation | materials
```

### 7.4 ROS2 bridge

Use a sidecar-first architecture for ROS2.

Reasons:

- ROS2 environments are dependency-heavy,
- Python/C++/DDS versions vary,
- robotics users often run on Linux with specific distributions,
- editor should not crash because a DDS participant misbehaves,
- sandboxing and process isolation are valuable.

Bridge responsibilities:

- connect to ROS2 graph,
- list nodes/topics/services/actions,
- subscribe/publish with QoS,
- convert messages to Create3D data streams,
- maintain TF frame graph,
- expose services/actions as Create3D tools,
- publish scene-driven commands/goals,
- provide diagnostics and latency stats.

Create3D communication with bridge:

- local IPC: gRPC/Unix socket/named pipe,
- remote: gRPC/WebSocket over TLS,
- message schemas: stable Create3D robotics schema, not raw arbitrary DDS in editor core.

### 7.5 ROS2 data mapping

| ROS2 concept | Create3D concept |
|---|---|
| Node | live endpoint entity/diagnostic object |
| Topic | LiveBinding stream |
| Service | callable robotics tool |
| Action | long-running tool with feedback/result |
| Parameter | editable component field or bridge config |
| TF frame | Transform component / frame graph node |
| JointState | RobotJoint live state |
| PointCloud2 | PointCloud stream/chunk source |
| Image | Texture/video stream |
| LaserScan | 2D/3D sensor visualization |
| OccupancyGrid | map layer / texture / grid asset |
| Path | curve/trajectory component |
| MarkerArray | debug/annotation entities |

### 7.6 URDF import

URDF importer pipeline:

1. parse XML or accept pre-expanded Xacro output,
2. normalize package paths,
3. import link visual meshes,
4. import collision meshes,
5. read inertials,
6. create entity per link,
7. create joint components,
8. build kinematic tree,
9. create robot root entity,
10. bind materials,
11. validate missing meshes/limits/inertials,
12. optionally create ROS2 topic bindings.

URDF entity structure:

```text
RobotRoot
  LinkEntity(base_link)
    visual mesh children
    collision proxy children
  LinkEntity(...)
  Joint components connect links
```

### 7.7 MJCF support

MJCF should be supported as both import and simulator adapter format.

Mapping:

| MJCF | Create3D |
|---|---|
| worldbody | scene/robot root |
| body | entity with transform/inertial |
| joint | RobotJoint component |
| geom | visual/collision geometry |
| actuator | actuator component |
| sensor | sensor component/live binding |
| material | material asset |
| asset mesh/texture | AssetDB entries |

Initial goal: import/view/convert. Later goal: edit/export and round-trip fidelity.

### 7.8 USD support for digital twin

USD should be a key interchange format, especially for digital twin and industrial scenes.

Create3D should support:

- import/export USD scenes,
- preserve references and hierarchy when possible,
- map USD prims to SceneDB entities,
- map variants/layers to Create3D variants/layers where possible,
- import meshes/materials/cameras/lights/animation,
- export scene snapshots to USD.

Internal SceneDB should not simply be USD. Create3D needs typed operations, collaboration, AI tools, robotics live bindings, and GPU cache metadata that do not map cleanly to raw USD. USD is a first-class interchange/composition target, not necessarily the internal database.

### 7.9 Nav2 integration

Nav2 workflows:

- show robot pose,
- show map and costmaps,
- send navigation goals,
- display planned path and controller trajectory,
- visualize obstacles,
- inspect behavior tree/status,
- record/replay navigation sessions,
- generate test scenes and blocked-path scenarios.

Create3D components:

- NavGoal,
- NavPath,
- CostmapLayer,
- RobotPose,
- NavigationSession,
- BehaviorStatus.

AI Robotics Agent examples:

- “Find why Nav2 cannot plan to this goal.”
- “Show all obstacles near the local costmap inflation radius.”
- “Generate three blocked-corridor scenarios for regression testing.”

### 7.10 Autoware integration

Autoware workflows:

- visualize vehicle model,
- stream point clouds,
- show perception objects,
- show lane/map data,
- show planned trajectories,
- inspect control commands,
- visualize diagnostics,
- build scenario scenes,
- compare recorded runs.

Create3D should not reimplement Autoware. It should be a visual scene, debugging, digital twin, and scenario-authoring workbench around Autoware.

### 7.11 Real-time coupling model

Robotics has multiple clocks:

- editor wall clock,
- simulation clock,
- ROS time,
- recorded bag time,
- animation timeline time.

Create3D needs a `ClockService` that supports:

- live mode,
- paused mode,
- simulation mode,
- replay mode,
- time scaling,
- timestamp alignment,
- interpolation/extrapolation.

Data flow:

```text
ROS2 Bridge Thread/Process
→ lock-free/ring-buffer message queue
→ time alignment
→ live component update transaction or transient live state
→ ECS projection
→ renderer/robotics panels
```

Important distinction:

- high-frequency sensor streams should not create persistent SceneDB transactions every frame,
- persistent events such as importing a robot, saving a goal, recording a session, or committing a reconstructed map should create transactions.

### 7.12 Digital twin architecture

Digital twin features:

- reality capture import,
- point cloud processing,
- semantic labeling,
- asset replacement from scans,
- live sensor streams,
- state overlays,
- time-series data attachment,
- simulation-vs-reality comparison,
- annotations and tasks,
- cloud review.

Digital twin data layers:

| Layer | Data |
|---|---|
| Static geometry | building, machines, terrain, roads |
| Captured reality | point clouds, meshes, Gaussian splats, images |
| Semantic layer | rooms, lanes, doors, obstacles, equipment |
| Robotics layer | robots, sensors, paths, costmaps, TF |
| Live state | telemetry, sensor streams, diagnostics |
| Simulation layer | physics, planned paths, scenario actors |
| Collaboration layer | comments, issues, approvals |

---

## 8. Technology Selection

### 8.1 Language: Rust vs C++ vs Python

| Language | Strengths | Weaknesses | Create3D decision |
|---|---|---|---|
| Rust | memory safety, performance, concurrency, modern package system, WASM fit, strong module boundaries | ecosystem gaps in DCC/CAD, learning curve, Rust ABI not stable for binary plugins | Primary language for core, scene, asset, renderer abstraction, editor, tools, server components |
| C++ | mature graphics/CAD/DCC/robotics ecosystems, existing libraries, maximum native interoperability | memory safety risk, build complexity, ABI/platform complexity, slower iteration | Use for optional bridges to mature libraries: CAD kernel, USD, ROS2 sidecar, specialized native plugins |
| Python | huge AI/science ecosystem, easy scripting, DCC familiarity | runtime performance, packaging, GIL/concurrency concerns, unsafe plugin behavior if unrestricted | Use for scripting, AI pipelines, notebooks, sidecars, not core engine state or critical renderer |

Final decision:

- **Rust is the product core.**
- **C++ is a bridge language.**
- **Python is automation and AI glue.**

### 8.2 UI: egui vs Slint vs Qt

| UI toolkit | Strengths | Weaknesses | Create3D decision |
|---|---|---|---|
| egui | pure Rust, immediate mode, fast iteration, great for tools/debug panels, easy integration with wgpu | not ideal for highly polished complex retained UI, layout constraints for professional app scale | Use for MVP editor shell, inspectors, debug UI, prototypes |
| Slint | declarative Rust/C++/JS/Python UI, native-looking direction, better product UI potential | less proven for complex DCC editor than Qt, integration work with custom viewport needed | Evaluate for production panels after MVP; possible Phase 2/3 adoption |
| Qt | mature professional desktop UI, battle-tested, docking, widgets, localization | C++/licensing/build complexity, less Rust-native, heavy dependency | Avoid as core dependency; consider only if product UI requirements exceed Rust-native options |

Recommended UI stack:

- Phase 1: `winit + wgpu + egui`.
- Phase 2: introduce Create3D UI abstraction for commands/panels/layout.
- Phase 3: evaluate Slint for polished retained panels if egui becomes limiting.
- Always keep viewport and GPU canvas custom.

### 8.3 ECS: Bevy ECS vs Flecs

| ECS | Strengths | Weaknesses | Create3D decision |
|---|---|---|---|
| Bevy ECS | Rust-native, ergonomic, scheduler, components/systems model, active ecosystem | tied philosophically to Bevy patterns, not a DCC scene database, reflection/schema limitations need control | Use as initial runtime/editor ECS behind `c3d_ecs` wrapper |
| Flecs | mature C/C++ ECS, lightweight, high scale, modules, used standalone | C/Rust FFI boundary, not Rust-native, integration friction | Keep as benchmark/optional simulation backend; not primary in Phase 1 |

Final ECS strategy:

- Use Bevy ECS for Phase 1 runtime/editor systems.
- Do not expose Bevy ECS as public Create3D scene API.
- Keep SceneDB authoritative and component schemas independent.
- Wrap ECS so replacement remains possible.

### 8.4 Rendering: wgpu vs Vulkan

| Option | Strengths | Weaknesses | Create3D decision |
|---|---|---|---|
| wgpu | cross-platform Rust-native abstraction over Vulkan/Metal/D3D12/OpenGL/WebGPU, fast MVP, safer API | advanced graphics features may lag native APIs, ray tracing and bindless limits may constrain future renderer | Use as Phase 1 backend through internal RHI |
| Vulkan direct | maximum explicit control, advanced GPU features, strong for high-end renderer | major implementation complexity, separate Metal/DX12 strategy needed | Build native advanced backend after renderer architecture stabilizes |

Final rendering strategy:

- `c3d_rhi` is the only renderer-facing API.
- `c3d_rhi_wgpu` ships first.
- `c3d_rhi_vulkan` is a planned advanced backend.
- OpenGL is fallback only.

### 8.5 Recommended baseline stack

| Area | Recommendation |
|---|---|
| Core language | Rust |
| Workspace | Cargo workspace, strict crate boundaries |
| Async/server | Tokio, Axum or tonic as needed |
| UI MVP | winit + wgpu + egui |
| ECS | Bevy ECS behind wrapper |
| Renderer MVP | wgpu behind RHI |
| Advanced renderer | Vulkan backend later |
| Serialization | serde + compact binary format for heavy chunks; JSON/TOML for manifests |
| Asset metadata DB | SQLite or RocksDB; content-addressed blob storage |
| Geometry | custom mesh/point/splat core; optional external libraries via plugins |
| Physics | Rust-native physics initially; external simulator adapters later |
| AI | provider-agnostic model router + sidecar model servers |
| Collaboration | local operation log + sync server; CRDT/OT hybrid |
| Plugins | WASM + sidecar + trusted native C ABI |
| Robotics | ROS2 sidecar bridge; URDF/MJCF importers |
| Interchange | USD, glTF/GLB, PLY, LAS/LAZ/COPC, E57, URDF, MJCF |

---

## 9. Repository Architecture

### 9.1 Top-level repository

```text
create3d/
├── Cargo.toml
├── rust-toolchain.toml
├── LICENSE-APACHE
├── LICENSE-MIT
├── README.md
├── CONTRIBUTING.md
├── SECURITY.md
├── apps/
│   ├── create3d-desktop/
│   ├── create3d-cli/
│   ├── create3d-server/
│   ├── create3d-headless/
│   └── create3d-ros-bridge/
├── editor/
│   ├── c3d-editor-shell/
│   ├── c3d-editor-ui/
│   ├── c3d-viewport/
│   ├── c3d-gizmos/
│   ├── c3d-inspector/
│   ├── c3d-command-palette/
│   └── c3d-editor-panels/
├── engine/
│   ├── c3d-core/
│   ├── c3d-app/
│   ├── c3d-ecs/
│   ├── c3d-scheduler/
│   ├── c3d-events/
│   ├── c3d-commands/
│   ├── c3d-diagnostics/
│   ├── c3d-plugin-host/
│   └── c3d-time/
├── renderer/
│   ├── c3d-rhi/
│   ├── c3d-rhi-wgpu/
│   ├── c3d-rhi-vulkan/
│   ├── c3d-render-graph/
│   ├── c3d-render-world/
│   ├── c3d-renderer-raster/
│   ├── c3d-renderer-pathtrace/
│   ├── c3d-renderer-pointcloud/
│   ├── c3d-renderer-gsplat/
│   ├── c3d-material/
│   ├── c3d-shader/
│   └── shaders/
├── scene/
│   ├── c3d-scene-doc/
│   ├── c3d-scene-ops/
│   ├── c3d-scene-schema/
│   ├── c3d-transform/
│   ├── c3d-selection/
│   ├── c3d-undo/
│   ├── c3d-stage/
│   └── c3d-scene-query/
├── asset/
│   ├── c3d-asset-db/
│   ├── c3d-asset-import/
│   ├── c3d-asset-export/
│   ├── c3d-asset-preview/
│   └── c3d-asset-cache/
├── geometry/
│   ├── c3d-geometry-core/
│   ├── c3d-mesh/
│   ├── c3d-mesh-processing/
│   ├── c3d-brep/
│   ├── c3d-nurbs/
│   ├── c3d-sdf/
│   ├── c3d-geometry-convert/
│   └── c3d-spatial/
├── pointcloud/
│   ├── c3d-pointcloud-core/
│   ├── c3d-pointcloud-io/
│   ├── c3d-pointcloud-index/
│   ├── c3d-pointcloud-processing/
│   └── c3d-pointcloud-renderprep/
├── gsplat/
│   ├── c3d-gsplat-core/
│   ├── c3d-gsplat-io/
│   ├── c3d-gsplat-processing/
│   └── c3d-gsplat-renderprep/
├── animation/
│   ├── c3d-animation-core/
│   ├── c3d-timeline/
│   ├── c3d-rigging/
│   └── c3d-retarget/
├── physics/
│   ├── c3d-physics-core/
│   ├── c3d-collision/
│   └── c3d-sim-adapter/
├── procedural/
│   ├── c3d-proc-core/
│   ├── c3d-proc-graph/
│   ├── c3d-proc-nodes/
│   └── c3d-proc-cache/
├── ai/
│   ├── c3d-ai-core/
│   ├── c3d-ai-tool-protocol/
│   ├── c3d-ai-context/
│   ├── c3d-ai-agent/
│   ├── c3d-ai-model-router/
│   ├── c3d-ai-memory/
│   ├── c3d-ai-sandbox/
│   └── c3d-ai-builtins/
├── robotics/
│   ├── c3d-robotics-core/
│   ├── c3d-robot-model/
│   ├── c3d-urdf/
│   ├── c3d-mjcf/
│   ├── c3d-ros2-client/
│   ├── c3d-nav2/
│   ├── c3d-autoware/
│   └── c3d-robotics-ui/
├── collaboration/
│   ├── c3d-collab-core/
│   ├── c3d-sync/
│   ├── c3d-presence/
│   ├── c3d-comments/
│   ├── c3d-branching/
│   └── c3d-authz/
├── plugins/
│   ├── c3d-plugin-api/
│   ├── c3d-plugin-wasm/
│   ├── c3d-plugin-native/
│   ├── c3d-plugin-python/
│   ├── importers/
│   ├── exporters/
│   └── examples/
├── sdk/
│   ├── rust/
│   ├── python/
│   ├── typescript/
│   ├── c/
│   └── schemas/
├── cloud/
│   ├── c3d-cloud-api/
│   ├── c3d-cloud-sync/
│   ├── c3d-cloud-assets/
│   ├── c3d-cloud-render/
│   └── deploy/
├── tests/
│   ├── golden-scenes/
│   ├── assets/
│   ├── integration/
│   ├── performance/
│   └── fuzz/
├── benches/
├── tools/
│   ├── xtask/
│   ├── schema-gen/
│   ├── shader-build/
│   └── asset-pack/
└── docs/
    ├── architecture/
    ├── rfcs/
    ├── plugin-guide/
    ├── ai-tools/
    ├── robotics/
    ├── rendering/
    ├── geometry/
    └── roadmap/
```

### 9.2 Dependency rules

Hard rules:

1. `engine/c3d-core` depends on no Create3D high-level crate.
2. `scene` may depend on `engine/c3d-core`, math, schema, asset IDs, but not editor or renderer backend.
3. `renderer` may depend on scene query types and geometry render caches, but not editor UI.
4. `editor` depends on engine, scene, renderer, asset, geometry, AI, robotics, collaboration.
5. `ai` depends on scene operations and tool protocol, not renderer internals.
6. `robotics` depends on scene, transform, asset, pointcloud, and optional sidecar protocol, not editor UI.
7. `plugins` depend on SDK/API crates, not internal unstable crates unless built-in.
8. `cloud` depends on sync/asset/server APIs, not desktop UI.

### 9.3 Dependency graph

```text
c3d-core
  ↓
c3d-schema / c3d-events / c3d-time / c3d-diagnostics
  ↓
c3d-scene-doc ← c3d-asset-db ← geometry/pointcloud/gsplat assets
  ↓              ↓
c3d-scene-ops   import/export/preview
  ↓
c3d-ecs projection
  ↓
c3d-render-world → c3d-render-graph → c3d-rhi → c3d-rhi-wgpu
  ↓
editor viewport/tools

ai-tool-protocol → scene-ops / asset-db / robotics-tools
robotics-core → scene-doc / transform / pointcloud / sidecar protocol
collab-core → scene-ops / op-log / authz
```

### 9.4 Public API boundaries

Stable public APIs:

- scene operation schema,
- component schema format,
- asset manifest format,
- plugin manifest format,
- AI tool protocol,
- robotics bridge protocol,
- SDK command/tool APIs.

Unstable internal APIs:

- renderer backend internals,
- ECS backend details,
- cache layouts,
- importer internals,
- UI implementation details,
- task scheduler internals.

### 9.5 Testing architecture

Test categories:

| Test type | Purpose |
|---|---|
| Unit | math, IDs, schemas, ops, importers |
| Golden scene | load/save/replay deterministic scenes |
| Renderer image | compare reference viewport output within tolerances |
| GPU capability | backend feature tests |
| Geometry property | mesh invariants, half-edge validity, SDF signs |
| Fuzz | importers, scene ops, serialization |
| Collaboration | concurrent operation merge/replay |
| AI tool | schema validation, permission checks, dry-run execution |
| Robotics | URDF/MJCF import, TF mapping, ROS bridge mock |
| Performance | large scenes, point cloud streaming, splat rendering, startup time |

### 9.6 OSS governance

Recommended:

- license: dual MIT/Apache-2.0 for Rust ecosystem friendliness,
- DCO sign-off or lightweight CLA; DCO is simpler for OSS adoption,
- public RFC process for core architecture changes,
- strict security policy for plugin/importer vulnerabilities,
- roadmap labels: MVP, Alpha, Beta, Good First Issue, AI Tools, Renderer, Geometry, Robotics,
- architecture decision records in `docs/rfcs/`.

---

## 10. Development Roadmap

### Phase 1 — MVP Foundation, months 0–3

Goal: a working desktop app that opens a project, shows a viewport, imports basic mesh assets, edits transforms, saves/loads SceneDB, and proves architecture.

Deliverables:

- Rust workspace and CI,
- desktop editor shell,
- SceneDB v0,
- transaction/undo system v0,
- Bevy ECS projection v0,
- wgpu RHI v0,
- raster viewport v0,
- glTF/GLB import v0,
- material basic PBR v0,
- transform gizmo,
- selection and inspector,
- asset database v0,
- command palette,
- headless scene load/save tests,
- architecture docs and RFC process.

Exit criteria:

- import GLB,
- create objects,
- select/move/rotate/scale,
- assign material,
- save/reopen project,
- undo/redo works,
- 60 FPS on simple scene,
- CI passes on Linux/macOS/Windows where feasible.

### Phase 2 — Alpha Authoring Platform, months 4–6

Goal: become a usable lightweight 3D scene editor with extensible tools.

Deliverables:

- mesh authoring data structure v1,
- basic mesh edit operations,
- asset import pipeline with thumbnails,
- material graph v0,
- render graph v1,
- viewport modes: solid/material/wireframe/id,
- plugin manifest v0,
- WASM plugin prototype,
- procedural graph v0,
- collaboration operation log prototype,
- AI tool protocol v0,
- AI Copilot read-only scene Q&A prototype,
- CLI import/export/thumbnail commands.

Exit criteria:

- edit mesh primitives,
- import/manage assets,
- run simple procedural graph,
- plugin can add a command/tool,
- AI can inspect scene and propose a transform/material transaction,
- operation log can replay scene.

### Phase 3 — Beta Core Differentiators, months 7–12

Goal: prove Create3D’s differentiated value: point clouds, Gaussian splats, AI transactions, robotics bridge, and collaboration.

Deliverables:

- point cloud import/stream/render v1,
- point cloud selection/crop/downsample,
- Gaussian splat import/render v1,
- splat selection/crop/LOD prototype,
- AI Copilot write transactions with approval,
- Modeling Agent v0,
- Scene Agent v0,
- ROS2 bridge sidecar v0,
- URDF import v1,
- TF and JointState visualization,
- Nav2 visualization prototype,
- collaboration sync server prototype,
- comments/presence,
- path tracing prototype or high-quality preview mode,
- export glTF/USD snapshot prototype.

Exit criteria:

- open a large point cloud with streaming LOD,
- view Gaussian splat scene interactively,
- AI can execute approved typed edits,
- import URDF and visualize live joint states/TF through bridge,
- two clients can see presence/comments and sync simple scene ops,
- Alpha release usable by early OSS users.

### Phase 4 — Production Studio, months 13–24

Goal: production-grade stability, collaboration, AI workflow, robotics/digital twin usability, and ecosystem.

Deliverables:

- robust sync/branch/merge,
- asset server and team workspace,
- GPU-driven renderer improvements,
- advanced material graph,
- path tracing with denoise,
- native Vulkan advanced backend evaluation,
- point cloud classification/registration,
- Gaussian splat advanced renderer and editing,
- robotics session recording/replay,
- MJCF support,
- Autoware support,
- USD import/export v1,
- plugin SDK v1,
- cloud remote render/preview,
- security hardening,
- installers/package releases.

Exit criteria:

- Beta users can complete real projects,
- plugin authors can build extensions,
- robotics users can debug ROS2 scenes,
- digital twin users can load scan data,
- teams can collaborate reliably.

### Phase 5 — Ecosystem and Scale, months 24+

Goal: Create3D becomes an extensible platform.

Deliverables:

- marketplace/registry for plugins and assets,
- web viewer/editor subset,
- distributed asset processing,
- remote GPU workspaces,
- advanced B-Rep/CAD workflows,
- neural reconstruction pipelines,
- scenario generation for robotics/simulation,
- enterprise/on-prem sync server,
- full public SDK compatibility guarantees.

---

## 11. Codex Implementation Plan: First 12 Months

This section is written as an implementation sequence suitable for Codex-driven OSS development. Each month has epics, concrete tasks, and acceptance criteria.

### Month 1 — Repository, core primitives, CI

Epics:

1. Workspace bootstrap.
2. Core crate foundation.
3. CI and quality gates.
4. Architecture docs.

Tasks:

- create Cargo workspace,
- add formatting/lint/test CI,
- define crate naming conventions,
- implement `c3d-core` primitives:
  - stable IDs,
  - result/error types,
  - math re-exports or math wrapper,
  - logging/tracing setup,
  - feature flags,
  - versioning constants,
- create `xtask` for common dev commands,
- create docs skeleton,
- add RFC template,
- add contribution guide,
- add security policy,
- create golden test harness skeleton.

Acceptance criteria:

- repository builds on clean machine,
- CI runs fmt, lint, unit tests,
- `cargo test --workspace` passes,
- docs define architecture rules and crate dependency policy.

### Month 2 — SceneDB v0 and transactions

Epics:

1. Scene document model.
2. Component schema registry.
3. Operations and transaction manager.
4. Save/load.

Tasks:

- implement SceneDoc entity table,
- implement stable entity IDs,
- implement Transform component schema,
- implement Name component schema,
- implement basic MeshRef/MaterialBinding component schemas as placeholders,
- implement create/delete entity operations,
- implement set component operation,
- implement transform operation,
- implement transaction apply/revert,
- implement undo/redo stack,
- implement scene serialization v0,
- implement scene replay tests,
- implement schema versioning/migration stubs.

Acceptance criteria:

- can create a scene with entities/transforms,
- can serialize/deserialize,
- operation log replay yields identical scene,
- undo/redo works for create/delete/transform/set-component,
- golden scene tests pass.

### Month 3 — Desktop shell and viewport MVP

Epics:

1. Desktop app.
2. RHI wgpu backend v0.
3. Basic renderer.
4. Scene projection.

Tasks:

- create `create3d-desktop`,
- set up window/event loop,
- integrate wgpu device/surface,
- define `c3d-rhi` traits v0,
- implement `c3d-rhi-wgpu`,
- implement basic render graph with clear pass,
- implement camera component,
- implement simple grid/axis render,
- implement ECS projection from SceneDB,
- implement basic renderable primitive cube/mesh placeholder,
- integrate egui overlay,
- add viewport panel,
- add frame timing diagnostics.

Acceptance criteria:

- app opens a window,
- viewport renders grid and a test cube,
- camera orbit/pan/zoom works,
- SceneDB entity transform updates visible object,
- no direct wgpu usage outside RHI backend except permitted bootstrap code.

### Month 4 — AssetDB and glTF import

Epics:

1. Asset database.
2. Mesh asset v0.
3. glTF/GLB import.
4. Basic material import.

Tasks:

- implement AssetID/content hash model,
- implement local asset blob storage,
- implement asset metadata manifest,
- implement import task system,
- implement glTF/GLB mesh import,
- implement texture import path,
- implement basic PBR material asset,
- implement scene import operation,
- implement asset preview placeholder,
- implement CLI command for import and scene creation,
- add importer fuzz/negative tests for malformed files where possible.

Acceptance criteria:

- user can import a GLB into a project,
- imported mesh appears in viewport,
- material base color/texture appears at basic level,
- project save/reload preserves asset references,
- CLI can import a GLB and produce a project.

### Month 5 — Editor interaction tools

Epics:

1. Selection.
2. Inspector.
3. Transform gizmo.
4. Command palette.

Tasks:

- implement selection state,
- implement ID buffer or CPU ray picking MVP,
- implement scene hierarchy panel,
- implement inspector panel for components,
- implement transform gizmo,
- implement snapping basics,
- implement command registry,
- implement command palette,
- implement keyboard shortcuts,
- integrate undo/redo with UI commands,
- add comments/annotations placeholder schema.

Acceptance criteria:

- user can select imported objects,
- transform through gizmo updates SceneDB transaction,
- inspector edits component fields,
- command palette can run registered commands,
- undo/redo works for UI actions.

### Month 6 — Mesh and material foundations

Epics:

1. Mesh authoring v0.
2. Mesh processing basics.
3. Material graph v0.
4. Renderer quality baseline.

Tasks:

- implement indexed mesh asset layout,
- implement half-edge mesh prototype or design-backed minimal topology layer,
- implement primitive creation: cube, plane, sphere/cylinder later,
- implement normals/tangents generation,
- implement mesh validation,
- implement simple edit operations: extrude/bevel can be prototype-level, or start with subdivide/delete face,
- implement material graph data model,
- implement material parameter inspector,
- implement viewport modes: solid, wireframe, material,
- implement thumbnail renderer MVP.

Acceptance criteria:

- user can create basic primitives,
- mesh validation catches invalid topology,
- material parameters can be edited,
- viewport modes switch correctly,
- thumbnails generated for mesh assets.

### Month 7 — Point Cloud Engine v0

Epics:

1. Point cloud asset format.
2. Point cloud import.
3. Chunked rendering.
4. Basic tools.

Tasks:

- implement PointCloudAsset metadata,
- implement PLY point cloud import first,
- add LAS/LAZ importer adapter plan and optional feature,
- implement octree/chunk index,
- implement point attribute storage,
- implement GPU point cloud renderer,
- implement LOD/residency manager prototype,
- implement crop box tool,
- implement attribute color modes: RGB, intensity, classification,
- add performance test dataset hooks.

Acceptance criteria:

- can import and view a point cloud,
- point cloud renders with LOD/chunks,
- crop tool creates a derived point cloud asset or scene filter,
- large point cloud test does not require loading all GPU data at once.

### Month 8 — Gaussian Splatting v0

Epics:

1. Splat asset model.
2. Importer.
3. Renderer.
4. Basic editing.

Tasks:

- implement GaussianSplatAsset metadata,
- implement PLY-based 3DGS importer,
- implement SoA splat storage,
- implement chunking/LOD stubs,
- implement screen-space splat projection shader path,
- implement tile/bin/sort MVP or approximate renderer,
- implement opacity/scale controls,
- implement selection/crop prototype,
- implement splat viewport mode,
- create golden sample scene.

Acceptance criteria:

- imported splat scene renders interactively on supported GPUs,
- opacity/scale can be adjusted,
- crop/selection works at prototype level,
- splat asset survives save/reload.

### Month 9 — AI Tool Protocol and Copilot v0

Epics:

1. AI tool schema.
2. Context builder.
3. Model router.
4. Copilot UI.
5. Transaction preview.

Tasks:

- define AI tool protocol schema,
- expose read-only scene query tools,
- expose safe write tools: create entity, transform, assign material,
- implement context pack builder,
- implement model provider interface,
- implement local/mock provider for tests,
- implement external provider plugin hook,
- implement Copilot chat panel,
- implement plan preview as temporary transaction,
- implement approval/commit flow,
- attach provenance to AI transactions,
- add AI tool permission checks.

Acceptance criteria:

- Copilot can answer questions about selected objects,
- Copilot can propose a transform/material edit,
- user can preview and approve AI-generated transaction,
- transaction records AI provenance,
- tests cover tool schema validation and permission denial.

### Month 10 — Robotics foundation

Epics:

1. Robot model schema.
2. URDF import.
3. ROS2 sidecar protocol.
4. TF/JointState visualization.

Tasks:

- implement RobotRoot/RobotLink/RobotJoint components,
- implement URDF parser/importer,
- implement link/joint hierarchy mapping,
- import visual/collision meshes through AssetDB,
- implement joint limit validation,
- define ROS2 bridge IPC protocol,
- implement bridge mock for tests,
- implement real ROS2 sidecar prototype,
- subscribe to TF and JointState,
- map live joint states to robot visualization,
- create robotics panel: topic list, TF tree, joint state.

Acceptance criteria:

- URDF robot imports into scene,
- kinematic hierarchy is visible,
- mock bridge updates joints live,
- real bridge can connect in a ROS2 environment,
- TF tree visualization works at basic level.

### Month 11 — Collaboration prototype

Epics:

1. Operation log sync.
2. Presence.
3. Comments.
4. Branch/proposal model.

Tasks:

- implement operation log persistence,
- implement sync server prototype,
- implement client sync protocol,
- implement presence cursors/selections,
- implement anchored comments,
- implement comment resolve/reopen,
- implement simple branch/proposal object,
- integrate AI proposals with branch/proposal UI,
- implement conflict policy for transforms and component set ops,
- lock destructive geometry ops for initial collaboration.

Acceptance criteria:

- two clients can open same test workspace and sync transforms,
- presence displays other user selection/cursor,
- comments attach to entities,
- conflicts handled deterministically for supported ops,
- unsupported ops fail safely or require lock.

### Month 12 — Alpha hardening and release

Epics:

1. Stability.
2. Performance.
3. Docs.
4. Packaging.
5. OSS launch readiness.

Tasks:

- profile startup and viewport,
- add large scene performance benchmarks,
- harden import errors,
- add crash-safe autosave/recovery,
- add project templates,
- improve UI polish,
- write user guide,
- write plugin guide v0,
- write AI tool guide,
- write robotics guide,
- package desktop builds,
- create sample projects:
  - mesh scene,
  - point cloud scene,
  - Gaussian splat scene,
  - URDF robot scene,
  - AI editing demo,
- tag Alpha release.

Acceptance criteria:

- new user can download/build and open samples,
- basic editing stable enough for public Alpha,
- docs cover architecture, build, plugin, AI, robotics,
- known limitations documented,
- issue templates and contribution paths ready.

---

## 12. Critical Design Risks and Mitigations

### 12.1 Risk: trying to clone Blender

Mitigation:

- Focus Phase 1 on architecture proof and differentiated workflows.
- Do not chase every modeling/animation feature.
- Use Blender interop rather than Blender replacement as a goal.

### 12.2 Risk: SceneDB/ECS confusion

Mitigation:

- Explicitly document SceneDB as authoritative.
- Keep ECS behind wrapper.
- Test scene replay independent of ECS.

### 12.3 Risk: renderer abstraction blocks advanced GPU features

Mitigation:

- Use `c3d_rhi` from day one.
- Keep wgpu backend isolated.
- Design feature flags for advanced native backend.
- Add Vulkan backend only after rendering architecture is stable.

### 12.4 Risk: AI produces low-quality or unsafe scene changes

Mitigation:

- Tool-only AI edits.
- Preview and approval.
- Validators.
- Provenance.
- Permissions.
- Local/offline mode.

### 12.5 Risk: point cloud and Gaussian data overwhelm editor

Mitigation:

- Chunking and streaming from first implementation.
- GPU residency manager.
- LOD metadata at import time.
- Avoid loading full datasets into SceneDB components.

### 12.6 Risk: robotics dependencies make desktop app fragile

Mitigation:

- Sidecar ROS2 bridge.
- Mock bridge tests.
- Optional features.
- Clear IPC schema.

### 12.7 Risk: plugin ABI instability

Mitigation:

- WASM/sidecar-first plugin model.
- C ABI for trusted native plugins.
- Internal Rust crates are not public ABI.

### 12.8 Risk: collaboration merge complexity

Mitigation:

- Start with operation log sync.
- Support safe transform/component merges first.
- Lock destructive geometry edits initially.
- Introduce CRDT/OT only where semantics are well understood.

---

## 13. Branding Recommendation

The codename “Create3D” communicates the product idea clearly, but it is highly collision-prone. Before public OSS launch:

1. check GitHub organization and crate/package names,
2. check `.org`, `.dev`, `.io`, `.ai`, `.app`, and regional domains,
3. check npm/PyPI/Cargo package names,
4. check trademark databases in target jurisdictions,
5. search App Store/Google Play/social handles,
6. choose a name with strong search uniqueness.

Possible naming directions:

- invented word + 3D meaning,
- “scene” or “world” root rather than generic “create”,
- Rust/GPU/agent connotation without being too technical,
- short CLI-friendly name.

Keep `create3d` as internal codename until brand validation finishes.

---

## 14. Source Notes Checked for Current Context

The architecture above is primarily a design proposal. Current ecosystem facts were checked against official or primary sources where relevant:

- Blender official site and features pages for Blender positioning.
- Autodesk Maya official product page for Maya positioning.
- SideFX Houdini official site for Houdini positioning.
- Unity official site for Unity positioning.
- Unreal Engine official site for Unreal positioning.
- Rust official site for Rust positioning.
- Python official site for Python positioning.
- ISO C++ references for C++ standard context.
- wgpu docs.rs page for backend support.
- Khronos/Vulkan official docs for Vulkan context.
- Apple Metal documentation for Metal context.
- Microsoft Direct3D 12 documentation for DirectX 12 context.
- ROS/ROS2 official documentation for ROS2 concepts.
- ROS URDF documentation for URDF context.
- MuJoCo documentation for MJCF context.
- OpenUSD official site and Pixar OpenUSD repository for USD context.
- Autoware official site/repository for Autoware context.
- Nav2 official documentation/site for Nav2 context.
- Inria/arXiv/ACM 3D Gaussian Splatting paper pages for Gaussian splatting context.
- Automerge/Yjs official pages for collaboration/CRDT context.
- Public search results for Create3D naming collision risk.
