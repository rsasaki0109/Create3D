//! Create3D desktop editor shell.

use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use c3d_ai_copilot::{
    CopilotEngine, CopilotProposal, CopilotResponse, RemoteLlmConfig, RemoteLlmProvider,
};
use c3d_asset_material::{MaterialAssetData, MaterialGraphData};
use c3d_collab_core::{BranchProposal, CommentStatus, SceneComment};
use c3d_core::logging::{init_logging, LoggingConfig};
use c3d_core::math::{Vec2, Vec3};
use c3d_core::C3D_VERSION;
use c3d_core::{AssetId, EntityId, UlidGenerator};
use c3d_ecs::{project_scene_to_ecs, RuntimeWorld};
use c3d_editor_core::{CommandRegistry, SelectionState};
use c3d_import_gsplat::looks_like_gsplat_ply;
use c3d_mesh_authoring::PrimitiveKind;
use c3d_project::{Project, ProjectTemplate, RecoverySnapshot};
use c3d_rhi::Extent2D;
use c3d_rhi_wgpu::WgpuBackend;
use c3d_robotics_core::{
    apply_joint_state, apply_tf_tree, live_tf_tree_from_message, primary_robot_bridge_target,
    robot_tf_trees, BridgeMessage, JointStateUpdate, LiveTfFrameNode, MockBridge, SidecarClient,
    SidecarClientConfig, TfTreeMessage, TfTreeNode, TopicInfo, DEFAULT_SIDECAR_ADDR,
};
use c3d_scene_ops::{SceneOperation, Transaction, TransactionManager};
use c3d_scene_schema::{
    GaussianSplatRef, Name, PointCloudColorMode, PointCloudCropBox, PointCloudRef, RobotJointType,
    Transform, TransformOp,
};
use c3d_sync::{CollabStore, SyncClient, SyncClientConfig, SyncEvent};
use c3d_viewport::{
    gizmo_drag_delta, pick_entity, pick_gizmo_axis, GizmoDragState, MeshGpuCache, OrbitCamera,
    PointCloudGpuCache, SplatGpuCache, ViewportRenderer, ViewportShadingMode,
};
use egui_wgpu::wgpu;
use egui_wgpu::{Renderer as EguiRenderer, ScreenDescriptor};
use egui_winit::State as EguiWinitState;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

fn main() {
    init_logging(&LoggingConfig::default());
    let event_loop = EventLoop::new().expect("create event loop");
    let mut app = DesktopApp::default();
    event_loop.run_app(&mut app).expect("run event loop");
}

struct DesktopApp {
    window: Option<Arc<Window>>,
    backend: Option<WgpuBackend>,
    egui_ctx: Option<egui::Context>,
    egui_winit: Option<EguiWinitState>,
    egui_renderer: Option<EguiRenderer>,
    viewport: Option<ViewportRenderer>,
    viewport_texture: Option<egui::TextureId>,
    camera: OrbitCamera,
    runtime: RuntimeWorld,
    project: Option<Project>,
    mesh_cache: MeshGpuCache,
    point_cloud_cache: PointCloudGpuCache,
    splat_cache: SplatGpuCache,
    scene_manager: Option<TransactionManager>,
    ids: UlidGenerator,
    demo_entity: Option<EntityId>,
    selection: SelectionState,
    commands: CommandRegistry,
    gizmo_drag: Option<GizmoDragState>,
    palette_open: bool,
    palette_query: String,
    inspector: InspectorState,
    shading_mode: ViewportShadingMode,
    viewport_rect: egui::Rect,
    viewport_extent: Extent2D,
    last_frame: Instant,
    frame_ms: f32,
    copilot_engine: CopilotEngine,
    copilot_input: String,
    copilot_messages: Vec<CopilotMessage>,
    copilot_pending: Option<CopilotProposal>,
    copilot_preview_manager: Option<TransactionManager>,
    robotics_bridge_mode: RoboticsBridgeMode,
    robotics_mock_bridge: Option<MockBridge>,
    robotics_sidecar: Option<SidecarClient>,
    robotics_bridge_addr: String,
    robotics_robot_name: String,
    robotics_joint_names: Vec<String>,
    robotics_topics: Vec<TopicInfo>,
    robotics_joint_states: Vec<(String, f64)>,
    robotics_live_tf: Option<TfTreeMessage>,
    robotics_status: String,
    collab_user_name: String,
    collab_server_addr: String,
    collab_workspace: String,
    collab_status: String,
    collab_comment_input: String,
    sync_client: Option<SyncClient>,
    remote_presence: Vec<c3d_collab_core::UserPresence>,
    collab_comments: Vec<SceneComment>,
    collab_proposals: Vec<BranchProposal>,
    applying_remote_sync: bool,
    scene_dirty: bool,
    last_autosave: Instant,
    recovery_snapshot: Option<RecoverySnapshot>,
    project_status: String,
    copilot_api_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoboticsBridgeMode {
    Off,
    Mock,
    Sidecar,
}

#[derive(Debug, Clone)]
struct CopilotMessage {
    role: CopilotRole,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopilotRole {
    User,
    Assistant,
}

#[derive(Debug, Default)]
struct InspectorState {
    entity_id: Option<EntityId>,
    name: String,
    translation: Vec3,
    material_id: Option<AssetId>,
    base_color: [f32; 4],
    point_cloud_asset_id: Option<AssetId>,
    point_cloud_color_mode: PointCloudColorMode,
    gaussian_splat_asset_id: Option<AssetId>,
    gaussian_splat_opacity_scale: f32,
    gaussian_splat_size_scale: f32,
    crop_min: [f32; 3],
    crop_max: [f32; 3],
    robot_joint_name: Option<String>,
    robot_joint_type: Option<RobotJointType>,
    robot_joint_position: f64,
    robot_joint_lower: Option<f64>,
    robot_joint_upper: Option<f64>,
}

enum ImportRequest {
    Glb(PathBuf),
    PointCloudPly(PathBuf),
    GsplatPly(PathBuf),
    AutoPly(PathBuf),
    Urdf(PathBuf),
}

impl InspectorState {
    fn crop_filter_from_state(&self) -> PointCloudCropBox {
        PointCloudCropBox {
            min: self.crop_min,
            max: self.crop_max,
        }
    }
}

impl Default for DesktopApp {
    fn default() -> Self {
        Self {
            window: None,
            backend: None,
            egui_ctx: None,
            egui_winit: None,
            egui_renderer: None,
            viewport: None,
            viewport_texture: None,
            camera: OrbitCamera::default(),
            runtime: RuntimeWorld::default(),
            project: None,
            mesh_cache: MeshGpuCache::default(),
            point_cloud_cache: PointCloudGpuCache::default(),
            splat_cache: SplatGpuCache::default(),
            scene_manager: None,
            ids: UlidGenerator::default(),
            demo_entity: None,
            selection: SelectionState::new(),
            commands: CommandRegistry::default_commands(),
            gizmo_drag: None,
            palette_open: false,
            palette_query: String::new(),
            inspector: InspectorState::default(),
            shading_mode: ViewportShadingMode::Material,
            viewport_rect: egui::Rect::NOTHING,
            viewport_extent: Extent2D {
                width: 1280,
                height: 720,
            },
            last_frame: Instant::now(),
            frame_ms: 0.0,
            copilot_engine: CopilotEngine::configured(),
            copilot_input: String::new(),
            copilot_messages: Vec::new(),
            copilot_pending: None,
            copilot_preview_manager: None,
            copilot_api_key: std::env::var("CREATE3D_COPILOT_API_KEY").unwrap_or_default(),
            robotics_bridge_mode: RoboticsBridgeMode::Off,
            robotics_mock_bridge: None,
            robotics_sidecar: None,
            robotics_bridge_addr: std::env::var("CREATE3D_ROS2_BRIDGE_ADDR")
                .unwrap_or_else(|_| DEFAULT_SIDECAR_ADDR.into()),
            robotics_robot_name: String::new(),
            robotics_joint_names: Vec::new(),
            robotics_topics: Vec::new(),
            robotics_joint_states: Vec::new(),
            robotics_live_tf: None,
            robotics_status: String::new(),
            collab_user_name: "Editor".into(),
            collab_server_addr: "127.0.0.1:9731".into(),
            collab_workspace: "default-workspace".into(),
            collab_status: String::new(),
            collab_comment_input: String::new(),
            sync_client: None,
            remote_presence: Vec::new(),
            collab_comments: Vec::new(),
            collab_proposals: Vec::new(),
            applying_remote_sync: false,
            scene_dirty: false,
            last_autosave: Instant::now(),
            recovery_snapshot: None,
            project_status: String::new(),
        }
    }
}

impl ApplicationHandler for DesktopApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Create3D")
                        .with_inner_size(LogicalSize::new(1280.0, 800.0)),
                )
                .expect("create window"),
        );

        let mut backend = WgpuBackend::from_window(window.clone()).expect("init wgpu backend");
        let size = window.inner_size();
        let extent = Extent2D {
            width: size.width.max(1),
            height: size.height.max(1),
        };

        let egui_ctx = egui::Context::default();
        let egui_winit = EguiWinitState::new(
            egui_ctx.clone(),
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let surface_format = backend.handles().surface_format;
        let mut egui_renderer = EguiRenderer::new(
            backend.handles().device,
            surface_format,
            Default::default(),
            1,
            true,
        );

        let viewport = ViewportRenderer::new(&mut backend, extent).expect("init viewport");
        let viewport_texture = {
            let handles = backend.handles();
            let target_resources = backend
                .target_resources(viewport.target())
                .expect("viewport target");
            egui_renderer.register_native_texture(
                handles.device,
                &target_resources.color_view,
                wgpu::FilterMode::Linear,
            )
        };

        let project_dir = default_project_dir();
        let (project, demo_entity) = open_or_create_desktop_project(&project_dir);
        let scene_manager = TransactionManager::new(project.scene().clone());
        if let Some(entity_id) = demo_entity {
            self.selection.select(entity_id);
        }
        project_scene_to_ecs(scene_manager.scene(), &mut self.runtime);
        self.recovery_snapshot = project.recovery_is_newer().ok().and_then(|is_newer| {
            if is_newer {
                project.recovery_snapshot().ok().flatten()
            } else {
                None
            }
        });

        self.window = Some(window);
        self.backend = Some(backend);
        self.egui_ctx = Some(egui_ctx);
        self.egui_winit = Some(egui_winit);
        self.egui_renderer = Some(egui_renderer);
        self.viewport = Some(viewport);
        self.viewport_texture = Some(viewport_texture);
        self.scene_manager = Some(scene_manager);
        self.project = Some(project);
        if let Some(project) = self.project.as_ref() {
            if let Ok(store) = CollabStore::load(project.root().join("collab")) {
                self.collab_comments = store.comments();
                self.collab_proposals = store.proposals();
            }
        }
        self.demo_entity = demo_entity;
        self.last_frame = Instant::now();
        self.last_autosave = Instant::now();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let Some(egui_winit) = self.egui_winit.as_mut() else {
            return;
        };
        let Some(egui_ctx) = self.egui_ctx.as_ref() else {
            return;
        };

        let response = egui_winit.on_window_event(&window, &event);
        if response.consumed {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(backend) = self.backend.as_mut() {
                    backend.resize_surface(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::MouseWheel { delta, .. } => {
                if self
                    .viewport_rect
                    .contains(egui_ctx.pointer_latest_pos().unwrap_or_default())
                {
                    let scroll = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(delta) => delta.y as f32 * 0.01,
                    };
                    self.camera.zoom(scroll);
                    self.sync_runtime();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl DesktopApp {
    fn sync_runtime(&mut self) {
        let scene = self
            .copilot_preview_manager
            .as_ref()
            .map(|manager| manager.scene())
            .or_else(|| self.scene_manager.as_ref().map(|manager| manager.scene()));
        if let Some(scene) = scene {
            project_scene_to_ecs(scene, &mut self.runtime);
        }
        if self.copilot_preview_manager.is_none() {
            if let (Some(project), Some(manager)) =
                (self.project.as_mut(), self.scene_manager.as_ref())
            {
                *project.scene_mut() = manager.scene().clone();
            }
        }
    }

    fn world_origin(&mut self, entity_id: EntityId) -> Option<Vec3> {
        self.runtime
            .drawables()
            .into_iter()
            .find(|drawable| drawable.entity_id == entity_id)
            .map(|drawable| drawable.world.transform_point3(Vec3::ZERO))
    }

    fn undo(&mut self) {
        if let Some(manager) = self.scene_manager.as_mut() {
            let _ = manager.undo();
            self.sync_runtime();
        }
    }

    fn redo(&mut self) {
        if let Some(manager) = self.scene_manager.as_mut() {
            let _ = manager.redo();
            self.sync_runtime();
        }
    }

    fn clear_selection(&mut self) {
        self.selection.clear();
        self.gizmo_drag = None;
    }

    fn focus_selection(&mut self) {
        let Some(entity_id) = self.selection.primary() else {
            return;
        };
        if let Some(origin) = self.world_origin(entity_id) {
            self.camera.focus_on(origin);
        }
    }

    fn run_command(&mut self, command_id: &str) -> Option<ImportRequest> {
        match command_id {
            "edit.undo" => {
                self.undo();
                None
            }
            "edit.redo" => {
                self.redo();
                None
            }
            "scene.open_project" => {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.open_project_at(path);
                }
                None
            }
            "scene.export_glb" => {
                self.export_project_glb();
                None
            }
            "scene.export_usd" => {
                self.export_project_usd();
                None
            }
            "scene.export_ply" => {
                self.export_project_ply();
                None
            }
            "scene.export_gsplat" => {
                self.export_project_gsplat();
                None
            }
            "project.save" => {
                self.persist_project();
                self.project_status = "Project saved".into();
                None
            }
            "scene.import_glb" => rfd::FileDialog::new()
                .add_filter("glTF", &["gltf", "glb"])
                .pick_file()
                .map(ImportRequest::Glb),
            "scene.import_ply" => rfd::FileDialog::new()
                .add_filter("PLY", &["ply"])
                .pick_file()
                .map(ImportRequest::PointCloudPly),
            "scene.import_gsplat" => rfd::FileDialog::new()
                .add_filter("3DGS PLY", &["ply"])
                .pick_file()
                .map(ImportRequest::GsplatPly),
            "scene.import_urdf" => rfd::FileDialog::new()
                .add_filter("URDF", &["urdf", "xacro"])
                .pick_file()
                .map(ImportRequest::Urdf),
            "pointcloud.crop_derived" => {
                self.crop_selected_point_cloud();
                None
            }
            "gsplat.crop_derived" => {
                self.crop_selected_gaussian_splat();
                None
            }
            "view.focus_selection" => {
                self.focus_selection();
                None
            }
            "selection.clear" => {
                self.clear_selection();
                None
            }
            "mesh.create_cube" => {
                self.create_primitive(PrimitiveKind::UnitCube, "Cube");
                None
            }
            "mesh.create_plane" => {
                self.create_primitive(PrimitiveKind::Plane, "Plane");
                None
            }
            "asset.generate_thumbnail" => {
                self.generate_selected_thumbnail();
                None
            }
            _ => None,
        }
    }

    fn create_primitive(&mut self, kind: PrimitiveKind, name: &str) {
        let report = {
            let Some(project) = self.project.as_mut() else {
                return;
            };
            match project.create_primitive(&mut self.ids, kind, name) {
                Ok(report) => report,
                Err(err) => {
                    tracing::error!("create primitive failed: {err}");
                    return;
                }
            }
        };
        let scene = self.project.as_ref().expect("project").scene().clone();
        self.scene_manager = Some(TransactionManager::new(scene));
        self.selection.select(report.entity_id);
        self.mesh_cache.invalidate_all();
        self.point_cloud_cache.invalidate_all();
        self.splat_cache.invalidate_all();
        self.sync_runtime();
        if let Some(project) = self.project.as_ref() {
            let _ = project.save();
        }
    }

    fn generate_selected_thumbnail(&mut self) {
        let Some(entity_id) = self.selection.primary() else {
            return;
        };
        let Some(project) = self.project.as_ref() else {
            return;
        };
        let Some(entity) = project.scene().get(entity_id) else {
            return;
        };
        let Some(mesh_ref) = entity.mesh_ref.as_ref() else {
            return;
        };
        let Some(material_id) = entity
            .material_binding
            .as_ref()
            .map(|binding| binding.material_id)
        else {
            return;
        };
        match project.write_mesh_thumbnail(mesh_ref.asset_id, material_id) {
            Ok(path) => tracing::info!("wrote thumbnail to {}", path.display()),
            Err(err) => tracing::error!("thumbnail generation failed: {err}"),
        }
    }

    fn apply_material_color(&mut self, material_id: AssetId, color: [f32; 4]) {
        let base_color_texture = self
            .project
            .as_ref()
            .and_then(|project| project.material_asset(material_id).ok())
            .and_then(|data| data.base_color_texture);
        let update_result = {
            let Some(project) = self.project.as_mut() else {
                return;
            };
            let material = MaterialAssetData {
                version: 1,
                base_color: color,
                base_color_texture,
                graph: Some(MaterialGraphData::from_base_color(color)),
            };
            project.update_material(material_id, material)
        };
        if let Err(err) = update_result {
            tracing::error!("material update failed: {err}");
            return;
        }
        self.mesh_cache.invalidate_all();
        self.point_cloud_cache.invalidate_all();
        self.splat_cache.invalidate_all();
        self.sync_runtime();
        if let Some(project) = self.project.as_ref() {
            let _ = project.save();
        }
    }

    fn apply_transform_translate(&mut self, entity_id: EntityId, delta: Vec3) {
        if delta.length_squared() <= f32::EPSILON {
            return;
        }
        if self.scene_manager.is_some() {
            let tx_id = self.ids.next_transaction_id();
            self.apply_scene_transaction(Transaction::new(
                tx_id,
                vec![SceneOperation::TransformOp {
                    entity_id,
                    op: TransformOp::Translate(delta),
                }],
            ));
        }
    }

    fn handle_viewport_input(
        &mut self,
        response: &egui::Response,
        rect: egui::Rect,
        aspect: f32,
        viewport_size: Vec2,
    ) {
        let modifiers = response.ctx.input(|input| input.modifiers);
        let pointer_pos = response
            .interact_pointer_pos()
            .map(|pos| Vec2::new(pos.x - rect.min.x, pos.y - rect.min.y));

        if response.dragged_by(egui::PointerButton::Middle) {
            let delta = response.drag_delta();
            self.camera.pan(Vec2::new(delta.x, delta.y));
            self.sync_runtime();
        }

        if response.dragged_by(egui::PointerButton::Primary) && modifiers.alt {
            let delta = response.drag_delta();
            self.camera.orbit(Vec2::new(delta.x, delta.y));
            self.sync_runtime();
        }

        let Some(screen_pos) = pointer_pos else {
            if response.drag_stopped() {
                self.gizmo_drag = None;
            }
            return;
        };

        if let (Some(entity_id), Some(mut drag)) = (self.selection.primary(), self.gizmo_drag) {
            if response.dragged_by(egui::PointerButton::Primary) && !modifiers.alt {
                if let Some(origin) = self.world_origin(entity_id) {
                    let total = gizmo_drag_delta(
                        origin,
                        drag.axis,
                        &self.camera,
                        aspect,
                        drag.start_screen,
                        screen_pos,
                        viewport_size,
                    );
                    let incremental = total - drag.accumulated;
                    drag.accumulated = total;
                    self.gizmo_drag = Some(drag);
                    self.apply_transform_translate(entity_id, incremental);
                }
            }
        }

        if response.clicked_by(egui::PointerButton::Primary) && !modifiers.alt {
            if let Some(entity_id) = self.selection.primary() {
                if let Some(origin) = self.world_origin(entity_id) {
                    if let Some(axis) = pick_gizmo_axis(
                        origin,
                        &self.camera,
                        aspect,
                        screen_pos,
                        viewport_size,
                        12.0,
                    ) {
                        self.gizmo_drag = Some(GizmoDragState::new(axis, screen_pos));
                        return;
                    }
                }
            }

            let scene = self
                .scene_manager
                .as_ref()
                .expect("scene manager")
                .scene()
                .clone();
            let assets = self.project.as_ref().expect("project").assets();
            let drawables = self.runtime.drawables();
            if let Some(hit) = pick_entity(
                &scene,
                assets,
                &drawables,
                &self.camera,
                aspect,
                screen_pos,
                viewport_size,
            ) {
                self.selection.select(hit.entity_id);
            } else {
                self.selection.clear();
            }
            self.gizmo_drag = None;
        }

        if response.drag_stopped() {
            self.gizmo_drag = None;
        }
    }

    fn sync_inspector_from_selection(&mut self) {
        let Some(entity_id) = self.selection.primary() else {
            self.inspector.entity_id = None;
            return;
        };
        if self.inspector.entity_id == Some(entity_id) {
            return;
        }
        let Some(entity) = self
            .scene_manager
            .as_ref()
            .and_then(|manager| manager.scene().get(entity_id))
        else {
            return;
        };
        self.inspector.entity_id = Some(entity_id);
        self.inspector.name = entity
            .name
            .as_ref()
            .map(|name| name.value.clone())
            .unwrap_or_default();
        self.inspector.translation = entity.transform.translation;
        self.inspector.material_id = entity
            .material_binding
            .as_ref()
            .map(|binding| binding.material_id);
        self.inspector.base_color = self
            .project
            .as_ref()
            .and_then(|project| {
                self.inspector
                    .material_id
                    .and_then(|material_id| project.material_asset(material_id).ok())
            })
            .and_then(|material| material.resolved().ok())
            .map(|resolved| resolved.base_color)
            .unwrap_or([1.0, 1.0, 1.0, 1.0]);
        self.inspector.point_cloud_asset_id = None;
        self.inspector.gaussian_splat_asset_id = None;
        self.inspector.gaussian_splat_opacity_scale = 1.0;
        self.inspector.gaussian_splat_size_scale = 1.0;
        self.inspector.point_cloud_asset_id = entity
            .point_cloud_ref
            .as_ref()
            .map(|point_cloud| point_cloud.asset_id);
        self.inspector.gaussian_splat_asset_id = entity
            .gaussian_splat_ref
            .as_ref()
            .map(|gaussian_splat| gaussian_splat.asset_id);
        if let Some(point_cloud) = entity.point_cloud_ref.as_ref() {
            self.inspector.point_cloud_color_mode = point_cloud.color_mode;
            if let Some(crop) = point_cloud.crop_filter {
                self.inspector.crop_min = crop.min;
                self.inspector.crop_max = crop.max;
            } else if let Some(project) = self.project.as_ref() {
                if let Ok(metadata) = project.point_cloud_asset(point_cloud.asset_id) {
                    self.inspector.crop_min = metadata.bounds_min;
                    self.inspector.crop_max = metadata.bounds_max;
                }
            }
        }
        if let Some(gaussian_splat) = entity.gaussian_splat_ref.as_ref() {
            self.inspector.gaussian_splat_opacity_scale = gaussian_splat.opacity_scale;
            self.inspector.gaussian_splat_size_scale = gaussian_splat.size_scale;
            if let Some(crop) = gaussian_splat.crop_filter {
                self.inspector.crop_min = crop.min;
                self.inspector.crop_max = crop.max;
            } else if let Some(project) = self.project.as_ref() {
                if let Ok(metadata) = project.gaussian_splat_asset(gaussian_splat.asset_id) {
                    self.inspector.crop_min = metadata.bounds_min;
                    self.inspector.crop_max = metadata.bounds_max;
                }
            }
        }
        self.inspector.robot_joint_name = None;
        self.inspector.robot_joint_type = None;
        self.inspector.robot_joint_position = 0.0;
        self.inspector.robot_joint_lower = None;
        self.inspector.robot_joint_upper = None;
        if let Some(joint) = entity.robot_joint.as_ref() {
            self.inspector.robot_joint_name = Some(joint.joint_name.clone());
            self.inspector.robot_joint_type = Some(joint.joint_type);
            self.inspector.robot_joint_position = joint.position;
            if let Some(limits) = joint.limits {
                self.inspector.robot_joint_lower = Some(limits.lower);
                self.inspector.robot_joint_upper = Some(limits.upper);
            }
        }
    }

    fn draw_inspector(&mut self, ui: &mut egui::Ui) {
        ui.heading("Inspector");
        let Some(entity_id) = self.selection.primary() else {
            ui.label("No selection");
            return;
        };

        self.sync_inspector_from_selection();

        let mut name = self.inspector.name.clone();
        ui.label("Name");
        if ui.text_edit_singleline(&mut name).changed() {
            self.inspector.name = name.clone();
            if let Some(manager) = self.scene_manager.as_mut() {
                let _ = manager.apply(Transaction::new(
                    self.ids.next_transaction_id(),
                    vec![SceneOperation::SetName {
                        entity_id,
                        name: Name::new(name),
                    }],
                ));
                self.sync_runtime();
            }
        }

        ui.separator();
        ui.label("Translation");
        let mut translation = self.inspector.translation;
        let mut changed = false;
        ui.horizontal(|ui| {
            changed |= ui
                .add(egui::DragValue::new(&mut translation.x).speed(0.05))
                .changed();
            changed |= ui
                .add(egui::DragValue::new(&mut translation.y).speed(0.05))
                .changed();
            changed |= ui
                .add(egui::DragValue::new(&mut translation.z).speed(0.05))
                .changed();
        });
        if changed {
            self.inspector.translation = translation;
            if self.scene_manager.is_some() {
                let current = self
                    .scene_manager
                    .as_ref()
                    .and_then(|manager| {
                        manager
                            .scene()
                            .get(entity_id)
                            .map(|entity| entity.transform)
                    })
                    .unwrap_or(Transform::IDENTITY);
                let mut next = current;
                next.translation = translation;
                let tx_id = self.ids.next_transaction_id();
                self.apply_scene_transaction(Transaction::new(
                    tx_id,
                    vec![SceneOperation::SetTransform {
                        entity_id,
                        transform: next,
                    }],
                ));
            }
        }

        if let Some(material_id) = self.inspector.material_id {
            ui.separator();
            ui.label("Material");
            let mut color = self.inspector.base_color;
            let mut changed = false;
            ui.horizontal(|ui| {
                changed |= ui
                    .add(egui::DragValue::new(&mut color[0]).speed(0.01))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut color[1]).speed(0.01))
                    .changed();
                changed |= ui
                    .add(egui::DragValue::new(&mut color[2]).speed(0.01))
                    .changed();
            });
            if changed {
                self.inspector.base_color = color;
                self.apply_material_color(material_id, color);
            }
        }

        if let Some(asset_id) = self.inspector.point_cloud_asset_id {
            ui.separator();
            ui.label("Point Cloud");
            ui.monospace(format!("{asset_id}"));
            let mut color_mode = self.inspector.point_cloud_color_mode;
            egui::ComboBox::from_label("Color Mode")
                .selected_text(color_mode.label())
                .show_ui(ui, |ui| {
                    for mode in PointCloudColorMode::all() {
                        if ui
                            .selectable_value(&mut color_mode, mode, mode.label())
                            .clicked()
                        {
                            self.apply_point_cloud_color_mode(entity_id, asset_id, color_mode);
                        }
                    }
                });

            ui.label("Crop Filter");
            let mut crop_min = self.inspector.crop_min;
            let mut crop_max = self.inspector.crop_max;
            let mut crop_changed = false;
            ui.horizontal(|ui| {
                crop_changed |= ui
                    .add(egui::DragValue::new(&mut crop_min[0]).speed(0.05))
                    .changed();
                crop_changed |= ui
                    .add(egui::DragValue::new(&mut crop_min[1]).speed(0.05))
                    .changed();
                crop_changed |= ui
                    .add(egui::DragValue::new(&mut crop_min[2]).speed(0.05))
                    .changed();
            });
            ui.horizontal(|ui| {
                crop_changed |= ui
                    .add(egui::DragValue::new(&mut crop_max[0]).speed(0.05))
                    .changed();
                crop_changed |= ui
                    .add(egui::DragValue::new(&mut crop_max[1]).speed(0.05))
                    .changed();
                crop_changed |= ui
                    .add(egui::DragValue::new(&mut crop_max[2]).speed(0.05))
                    .changed();
            });
            if crop_changed {
                self.inspector.crop_min = crop_min;
                self.inspector.crop_max = crop_max;
                self.apply_point_cloud_crop_filter(
                    entity_id,
                    asset_id,
                    PointCloudCropBox {
                        min: crop_min,
                        max: crop_max,
                    },
                );
            }
            if ui.button("Create Derived Cropped Asset").clicked() {
                self.crop_selected_point_cloud();
            }
        }

        if let Some(asset_id) = self.inspector.gaussian_splat_asset_id {
            ui.separator();
            ui.label("Gaussian Splat");
            ui.monospace(format!("{asset_id}"));
            let mut opacity_scale = self.inspector.gaussian_splat_opacity_scale;
            let mut size_scale = self.inspector.gaussian_splat_size_scale;
            let mut scale_changed = false;
            scale_changed |= ui
                .add(
                    egui::DragValue::new(&mut opacity_scale)
                        .speed(0.01)
                        .range(0.0..=2.0)
                        .prefix("Opacity "),
                )
                .changed();
            scale_changed |= ui
                .add(
                    egui::DragValue::new(&mut size_scale)
                        .speed(0.01)
                        .range(0.1..=4.0)
                        .prefix("Size "),
                )
                .changed();
            if scale_changed {
                self.inspector.gaussian_splat_opacity_scale = opacity_scale;
                self.inspector.gaussian_splat_size_scale = size_scale;
                self.apply_gaussian_splat_scales(entity_id, asset_id, opacity_scale, size_scale);
            }

            ui.label("Crop Filter");
            let mut crop_min = self.inspector.crop_min;
            let mut crop_max = self.inspector.crop_max;
            let mut crop_changed = false;
            ui.horizontal(|ui| {
                crop_changed |= ui
                    .add(egui::DragValue::new(&mut crop_min[0]).speed(0.05))
                    .changed();
                crop_changed |= ui
                    .add(egui::DragValue::new(&mut crop_min[1]).speed(0.05))
                    .changed();
                crop_changed |= ui
                    .add(egui::DragValue::new(&mut crop_min[2]).speed(0.05))
                    .changed();
            });
            ui.horizontal(|ui| {
                crop_changed |= ui
                    .add(egui::DragValue::new(&mut crop_max[0]).speed(0.05))
                    .changed();
                crop_changed |= ui
                    .add(egui::DragValue::new(&mut crop_max[1]).speed(0.05))
                    .changed();
                crop_changed |= ui
                    .add(egui::DragValue::new(&mut crop_max[2]).speed(0.05))
                    .changed();
            });
            if crop_changed {
                self.inspector.crop_min = crop_min;
                self.inspector.crop_max = crop_max;
                self.apply_gaussian_splat_crop_filter(
                    entity_id,
                    asset_id,
                    PointCloudCropBox {
                        min: crop_min,
                        max: crop_max,
                    },
                );
            }
            if ui.button("Create Derived Cropped Asset").clicked() {
                self.crop_selected_gaussian_splat();
            }
        }

        if let Some(joint_name) = self.inspector.robot_joint_name.clone() {
            ui.separator();
            ui.label("Robot Joint");
            ui.monospace(&joint_name);
            if let Some(joint_type) = self.inspector.robot_joint_type {
                ui.label(format!("Type: {joint_type:?}"));
            }
            let mut position = self.inspector.robot_joint_position;
            let speed = match self.inspector.robot_joint_type {
                Some(RobotJointType::Prismatic) => 0.01,
                _ => 0.02,
            };
            if ui
                .add(
                    egui::DragValue::new(&mut position)
                        .speed(speed)
                        .prefix("Position "),
                )
                .changed()
            {
                self.apply_robot_joint_position(entity_id, &joint_name, position);
            }
            if let (Some(lower), Some(upper)) = (
                self.inspector.robot_joint_lower,
                self.inspector.robot_joint_upper,
            ) {
                ui.label(format!("Limits: [{lower:.3}, {upper:.3}]"));
            }
        }
    }

    fn draw_hierarchy(&mut self, ui: &mut egui::Ui) {
        ui.heading("Hierarchy");
        let entities: Vec<_> = self
            .scene_manager
            .as_ref()
            .map(|manager| manager.scene().entities().collect())
            .unwrap_or_default();
        for entity in entities {
            let label = entity
                .name
                .as_ref()
                .map(|name| name.value.as_str())
                .unwrap_or("<Unnamed>");
            let prefix = if entity.robot_root.is_some() {
                "[Robot] "
            } else if entity.robot_link.is_some() {
                "[Link] "
            } else if entity.robot_joint.is_some() {
                "[Joint] "
            } else {
                ""
            };
            let selected = self.selection.is_selected(entity.id);
            if ui
                .selectable_label(selected, format!("{prefix}{label}"))
                .clicked()
            {
                self.selection.select(entity.id);
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) -> Option<ImportRequest> {
        let mut import_request = None;
        ctx.input(|input| {
            if input.modifiers.ctrl && input.key_pressed(egui::Key::Z) && !input.modifiers.shift {
                self.undo();
            }
            if input.modifiers.ctrl
                && (input.key_pressed(egui::Key::Y)
                    || (input.key_pressed(egui::Key::Z) && input.modifiers.shift))
            {
                self.redo();
            }
            if input.key_pressed(egui::Key::F) {
                self.focus_selection();
            }
            if input.key_pressed(egui::Key::Escape) {
                self.clear_selection();
            }
            if input.modifiers.ctrl && input.modifiers.shift && input.key_pressed(egui::Key::P) {
                self.palette_open = true;
            }
        });

        if self.palette_open {
            let mut close_palette = false;
            let mut run_command = None;
            egui::Window::new("Command Palette")
                .collapsible(false)
                .default_width(420.0)
                .show(ctx, |ui| {
                    ui.label("Ctrl+Shift+P");
                    if ui
                        .text_edit_singleline(&mut self.palette_query)
                        .lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Escape))
                    {
                        close_palette = true;
                    }
                    for command in self.commands.search(&self.palette_query) {
                        let label = match command.shortcut {
                            Some(shortcut) => format!("{}  ({shortcut})", command.label),
                            None => command.label.to_string(),
                        };
                        if ui.button(label).clicked() {
                            run_command = Some(command.id);
                            close_palette = true;
                        }
                    }
                });
            if close_palette {
                self.palette_open = false;
                self.palette_query.clear();
            }
            if let Some(command_id) = run_command {
                import_request = self.run_command(command_id);
            }
        }

        import_request
    }

    fn import_glb(&mut self, path: PathBuf) {
        match Self::import_glb_inner(self.project.as_mut(), &mut self.ids, path) {
            Ok(report) => {
                let scene = self.project.as_ref().expect("project").scene().clone();
                self.scene_manager = Some(TransactionManager::new(scene));
                self.sync_runtime();
                if let Some(project) = self.project.as_ref() {
                    let _ = project.save();
                }
                tracing::info!(
                    "imported {} entities, {} meshes",
                    report.entity_count,
                    report.mesh_assets.len()
                );
            }
            Err(err) => tracing::error!("glb import failed: {err}"),
        }
    }

    fn import_glb_inner(
        project: Option<&mut Project>,
        ids: &mut UlidGenerator,
        path: PathBuf,
    ) -> Result<c3d_project::ImportReport, c3d_project::ProjectError> {
        let Some(project) = project else {
            return Err(c3d_project::ProjectError::NotFound("project".into()));
        };
        project.import_gltf(path, ids)
    }

    fn import_ply(&mut self, path: PathBuf) {
        let Some(project) = self.project.as_mut() else {
            return;
        };
        match project.import_ply(path, &mut self.ids) {
            Ok(report) => {
                let scene = self.project.as_ref().expect("project").scene().clone();
                self.scene_manager = Some(TransactionManager::new(scene));
                self.selection.select(report.entity_id);
                self.point_cloud_cache.invalidate_all();
                self.splat_cache.invalidate_all();
                self.sync_runtime();
                if let Some(project) = self.project.as_ref() {
                    let _ = project.save();
                }
                tracing::info!(
                    "imported point cloud with {} points in {} chunks",
                    report.point_count,
                    report.chunk_assets.len()
                );
            }
            Err(err) => tracing::error!("ply import failed: {err}"),
        }
    }

    fn import_gsplat(&mut self, path: PathBuf) {
        let Some(project) = self.project.as_mut() else {
            return;
        };
        match project.import_gsplat_ply(path, &mut self.ids) {
            Ok(report) => {
                let scene = self.project.as_ref().expect("project").scene().clone();
                self.scene_manager = Some(TransactionManager::new(scene));
                self.selection.select(report.entity_id);
                self.point_cloud_cache.invalidate_all();
                self.splat_cache.invalidate_all();
                self.sync_runtime();
                if let Some(project) = self.project.as_ref() {
                    let _ = project.save();
                }
                tracing::info!(
                    "imported gaussian splats with {} splats in {} chunks",
                    report.splat_count,
                    report.chunk_assets.len()
                );
            }
            Err(err) => tracing::error!("gsplat import failed: {err}"),
        }
    }

    fn import_urdf(&mut self, path: PathBuf) {
        let Some(project) = self.project.as_mut() else {
            return;
        };
        match project.import_urdf(path, &mut self.ids) {
            Ok(report) => {
                let scene = self.project.as_ref().expect("project").scene().clone();
                self.scene_manager = Some(TransactionManager::new(scene));
                self.selection.select(report.root_entity_id);
                self.mesh_cache.invalidate_all();
                self.refresh_robotics_targets();
                self.robotics_status = format!(
                    "Imported robot `{}` with {} links",
                    report.robot_name,
                    report.link_entities.len()
                );
                self.sync_runtime();
                if let Some(project) = self.project.as_ref() {
                    let _ = project.save();
                }
                tracing::info!(
                    "imported urdf robot {} with {} joints",
                    report.robot_name,
                    report.joint_names.len()
                );
            }
            Err(err) => tracing::error!("urdf import failed: {err}"),
        }
    }

    fn apply_robot_joint_position(&mut self, entity_id: EntityId, joint_name: &str, position: f64) {
        if let Some(manager) = self.scene_manager.as_mut() {
            match apply_joint_state(
                manager.scene_mut(),
                &JointStateUpdate {
                    joint_name: joint_name.to_string(),
                    position,
                },
            ) {
                Ok(_) => {
                    self.inspector.robot_joint_position = position;
                    self.sync_runtime();
                }
                Err(err) => tracing::warn!("joint update rejected: {err}"),
            }
        }
        let _ = entity_id;
    }

    fn refresh_robotics_targets(&mut self) {
        let Some(manager) = self.scene_manager.as_ref() else {
            return;
        };
        if let Some(target) = primary_robot_bridge_target(manager.scene()) {
            self.robotics_robot_name = target.robot_name.clone();
            self.robotics_joint_names = target.joint_names.clone();
            self.robotics_mock_bridge = Some(MockBridge::new(
                target.robot_name,
                target.joint_names.clone(),
            ));
            self.robotics_joint_states = target
                .joint_names
                .iter()
                .map(|name| (name.clone(), 0.0))
                .collect();
        } else {
            self.robotics_robot_name.clear();
            self.robotics_joint_names.clear();
            self.robotics_mock_bridge = None;
            self.robotics_joint_states.clear();
        }
    }

    fn sidecar_binary(&self) -> String {
        std::env::var("CREATE3D_ROS2_BRIDGE_BIN").unwrap_or_else(|_| "create3d-ros2-bridge".into())
    }

    fn spawn_sidecar_process(&self) -> Result<(), String> {
        let mut command = Command::new(self.sidecar_binary());
        command
            .arg("--listen")
            .arg(&self.robotics_bridge_addr)
            .arg("--robot-name")
            .arg(&self.robotics_robot_name)
            .arg("--joint-names")
            .arg(self.robotics_joint_names.join(","))
            .arg("--joint-states-topic")
            .arg(
                std::env::var("CREATE3D_ROS2_JOINT_STATES_TOPIC")
                    .unwrap_or_else(|_| "/joint_states".into()),
            )
            .arg("--tf-topic")
            .arg(std::env::var("CREATE3D_ROS2_TF_TOPIC").unwrap_or_else(|_| "/tf".into()))
            .arg("--tf-static-topic")
            .arg(
                std::env::var("CREATE3D_ROS2_TF_STATIC_TOPIC")
                    .unwrap_or_else(|_| "/tf_static".into()),
            )
            .arg("--tf-root-frame")
            .arg(std::env::var("CREATE3D_ROS2_TF_ROOT").unwrap_or_else(|_| "base_link".into()));
        if self.sidecar_disable_tf() {
            command.arg("--no-tf");
        }
        if self.sidecar_use_ros2() {
            command.arg("--ros2").arg("--no-mock");
        }
        command
            .spawn()
            .map(|_| ())
            .map_err(|err| format!("failed to spawn sidecar: {err}"))
    }

    fn sidecar_use_ros2(&self) -> bool {
        std::env::var("CREATE3D_ROS2_BRIDGE_ROS2")
            .ok()
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "YES" | "yes" | "True"))
    }

    fn sidecar_disable_tf(&self) -> bool {
        std::env::var("CREATE3D_ROS2_BRIDGE_NO_TF")
            .ok()
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "YES" | "yes" | "True"))
    }

    fn start_robotics_mock(&mut self) {
        if self.robotics_mock_bridge.is_none() {
            self.robotics_status = "Import a URDF robot before starting the mock bridge".into();
            return;
        }
        self.stop_robotics_bridge();
        self.robotics_bridge_mode = RoboticsBridgeMode::Mock;
        self.robotics_status = "Mock ROS2 bridge running".into();
    }

    fn start_robotics_sidecar(&mut self) {
        if self.robotics_robot_name.is_empty() || self.robotics_joint_names.is_empty() {
            self.robotics_status =
                "Import or open a URDF robot before starting the sidecar bridge".into();
            return;
        }
        self.stop_robotics_bridge();

        let config = SidecarClientConfig {
            server_addr: self.robotics_bridge_addr.clone(),
            client_id: "create3d-desktop".into(),
        };

        if let Ok(client) = SidecarClient::connect(config.clone()) {
            self.robotics_sidecar = Some(client);
            self.robotics_bridge_mode = RoboticsBridgeMode::Sidecar;
            self.robotics_status =
                format!("Sidecar bridge connected at {}", self.robotics_bridge_addr);
            return;
        }

        if let Err(err) = self.spawn_sidecar_process() {
            self.robotics_status = err;
            return;
        }

        std::thread::sleep(Duration::from_millis(250));
        match SidecarClient::connect(config) {
            Ok(client) => {
                self.robotics_sidecar = Some(client);
                self.robotics_bridge_mode = RoboticsBridgeMode::Sidecar;
                self.robotics_status = format!(
                    "Sidecar bridge spawned and connected at {}",
                    self.robotics_bridge_addr
                );
            }
            Err(err) => {
                self.robotics_status = format!("Sidecar spawned but connect failed: {err}");
            }
        }
    }

    fn stop_robotics_bridge(&mut self) {
        if let Some(mut client) = self.robotics_sidecar.take() {
            client.disconnect();
        }
        self.robotics_bridge_mode = RoboticsBridgeMode::Off;
        self.robotics_live_tf = None;
        self.robotics_status = "Robotics bridge stopped".into();
    }

    fn apply_bridge_envelopes(&mut self, envelopes: Vec<c3d_robotics_core::BridgeEnvelope>) {
        let mut joint_updates = Vec::new();
        let mut tf_update = None;
        for envelope in envelopes {
            match envelope.message {
                BridgeMessage::TopicList { topics } => {
                    self.robotics_topics = topics;
                }
                BridgeMessage::JointState(message) => {
                    self.robotics_joint_states = message
                        .joint_names
                        .iter()
                        .zip(message.positions.iter())
                        .map(|(name, position)| (name.clone(), *position))
                        .collect();
                    for (name, position) in &self.robotics_joint_states {
                        joint_updates.push(JointStateUpdate {
                            joint_name: name.clone(),
                            position: *position,
                        });
                    }
                }
                BridgeMessage::TfTree(message) => {
                    self.robotics_live_tf = Some(message.clone());
                    tf_update = Some(message);
                }
                BridgeMessage::Hello { .. } => {}
            }
        }
        if joint_updates.is_empty() && tf_update.is_none() {
            return;
        }
        if let Some(manager) = self.scene_manager.as_mut() {
            if !joint_updates.is_empty() {
                if let Err(err) =
                    c3d_robotics_core::apply_joint_states(manager.scene_mut(), &joint_updates)
                {
                    tracing::warn!("bridge joint update failed: {err}");
                }
            }
            if let Some(message) = tf_update {
                if let Err(err) = apply_tf_tree(manager.scene_mut(), &message) {
                    tracing::warn!("bridge tf update failed: {err}");
                }
            }
            self.sync_runtime();
        }
    }

    fn tick_robotics_bridge(&mut self) {
        match self.robotics_bridge_mode {
            RoboticsBridgeMode::Off => {}
            RoboticsBridgeMode::Mock => {
                let Some(bridge) = self.robotics_mock_bridge.as_mut() else {
                    return;
                };
                let envelopes = bridge.next_envelopes();
                self.apply_bridge_envelopes(envelopes);
            }
            RoboticsBridgeMode::Sidecar => {
                let envelopes = self
                    .robotics_sidecar
                    .as_mut()
                    .map(SidecarClient::poll_envelopes)
                    .unwrap_or_default();
                if envelopes.is_empty() {
                    return;
                }
                self.apply_bridge_envelopes(envelopes);
            }
        }
    }

    fn draw_robotics_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Robotics");
        if !self.robotics_status.is_empty() {
            ui.label(&self.robotics_status);
        }
        ui.horizontal(|ui| {
            if ui.button("Start Mock Bridge").clicked() {
                self.start_robotics_mock();
            }
            if ui.button("Start Sidecar Bridge").clicked() {
                self.start_robotics_sidecar();
            }
            if ui.button("Stop Bridge").clicked() {
                self.stop_robotics_bridge();
            }
        });
        ui.label(format!("Sidecar: {}", self.robotics_bridge_addr));

        ui.separator();
        ui.label("Topics");
        if self.robotics_topics.is_empty() {
            ui.label("No topics yet");
        } else {
            for topic in &self.robotics_topics {
                ui.monospace(format!("{} ({})", topic.name, topic.message_type));
            }
        }

        ui.separator();
        ui.label("Joint States");
        if self.robotics_joint_states.is_empty() {
            ui.label("No joint states yet");
        } else {
            for (name, position) in &self.robotics_joint_states {
                ui.label(format!("{name}: {position:.3}"));
            }
        }

        ui.separator();
        ui.label("TF Tree");
        if let Some(message) = self.robotics_live_tf.as_ref() {
            ui.label(format!(
                "Live TF ({} edges, root={})",
                message.edges.len(),
                message.root_frame
            ));
            let tree = live_tf_tree_from_message(message);
            self.draw_live_tf_node(ui, &tree, 0);
        } else {
            let trees = self
                .scene_manager
                .as_ref()
                .map(|manager| robot_tf_trees(manager.scene()))
                .unwrap_or_default();
            if trees.is_empty() {
                ui.label("No robot roots in scene");
            } else {
                for tree in trees {
                    ui.label(format!("Robot: {}", tree.robot_name));
                    self.draw_tf_node(ui, &tree.root, 0);
                }
            }
        }
    }

    fn draw_live_tf_node(&self, ui: &mut egui::Ui, node: &LiveTfFrameNode, depth: usize) {
        ui.label(format!(
            "{:indent$}{}",
            "",
            node.frame_name,
            indent = depth * 2
        ));
        for child in &node.children {
            self.draw_live_tf_node(ui, child, depth + 1);
        }
    }

    fn draw_tf_node(&self, ui: &mut egui::Ui, node: &TfTreeNode, depth: usize) {
        ui.label(format!(
            "{:indent$}{}",
            "",
            node.frame_name,
            indent = depth * 2
        ));
        for child in &node.children {
            self.draw_tf_node(ui, child, depth + 1);
        }
    }

    fn dispatch_import(&mut self, request: ImportRequest) {
        match request {
            ImportRequest::Glb(path) => self.import_glb(path),
            ImportRequest::PointCloudPly(path) => self.import_ply(path),
            ImportRequest::GsplatPly(path) => self.import_gsplat(path),
            ImportRequest::Urdf(path) => self.import_urdf(path),
            ImportRequest::AutoPly(path) => {
                let is_gsplat = std::fs::read(&path)
                    .ok()
                    .is_some_and(|bytes| looks_like_gsplat_ply(&bytes));
                if is_gsplat {
                    self.import_gsplat(path);
                } else {
                    self.import_ply(path);
                }
            }
        }
    }

    fn apply_gaussian_splat_scales(
        &mut self,
        entity_id: EntityId,
        asset_id: AssetId,
        opacity_scale: f32,
        size_scale: f32,
    ) {
        self.inspector.gaussian_splat_opacity_scale = opacity_scale;
        self.inspector.gaussian_splat_size_scale = size_scale;
        let gaussian_splat_ref = GaussianSplatRef {
            asset_id,
            opacity_scale,
            size_scale,
            crop_filter: Some(self.inspector.crop_filter_from_state()),
        };
        if let Some(manager) = self.scene_manager.as_mut() {
            let _ = manager.apply(Transaction::new(
                self.ids.next_transaction_id(),
                vec![SceneOperation::SetGaussianSplatRef {
                    entity_id,
                    gaussian_splat_ref,
                }],
            ));
            self.splat_cache.invalidate_all();
            self.sync_runtime();
        }
    }

    fn apply_gaussian_splat_crop_filter(
        &mut self,
        entity_id: EntityId,
        asset_id: AssetId,
        crop: PointCloudCropBox,
    ) {
        let gaussian_splat_ref = GaussianSplatRef {
            asset_id,
            opacity_scale: self.inspector.gaussian_splat_opacity_scale,
            size_scale: self.inspector.gaussian_splat_size_scale,
            crop_filter: Some(crop),
        };
        if let Some(manager) = self.scene_manager.as_mut() {
            let _ = manager.apply(Transaction::new(
                self.ids.next_transaction_id(),
                vec![SceneOperation::SetGaussianSplatRef {
                    entity_id,
                    gaussian_splat_ref,
                }],
            ));
            self.splat_cache.invalidate_all();
            self.sync_runtime();
        }
    }

    fn crop_selected_gaussian_splat(&mut self) {
        let Some(entity_id) = self.selection.primary() else {
            return;
        };
        let Some(project) = self.project.as_mut() else {
            return;
        };
        let Some(entity) = project.scene().get(entity_id) else {
            return;
        };
        let Some(gaussian_splat_ref) = entity.gaussian_splat_ref.clone() else {
            return;
        };
        let crop = self.inspector.crop_filter_from_state();
        match project.crop_gaussian_splat(
            gaussian_splat_ref.asset_id,
            crop,
            &mut self.ids,
            format!("{entity_id}-cropped"),
        ) {
            Ok(derived_id) => {
                if let Some(manager) = self.scene_manager.as_mut() {
                    let _ = manager.apply(Transaction::new(
                        self.ids.next_transaction_id(),
                        vec![SceneOperation::SetGaussianSplatRef {
                            entity_id,
                            gaussian_splat_ref: GaussianSplatRef {
                                asset_id: derived_id,
                                opacity_scale: gaussian_splat_ref.opacity_scale,
                                size_scale: gaussian_splat_ref.size_scale,
                                crop_filter: None,
                            },
                        }],
                    ));
                }
                self.splat_cache.invalidate_all();
                self.sync_runtime();
                if let Some(project) = self.project.as_ref() {
                    let _ = project.save();
                }
                tracing::info!("created derived cropped gaussian splat {derived_id}");
            }
            Err(err) => tracing::error!("crop gaussian splat failed: {err}"),
        }
    }

    fn apply_point_cloud_color_mode(
        &mut self,
        entity_id: EntityId,
        asset_id: AssetId,
        color_mode: PointCloudColorMode,
    ) {
        self.inspector.point_cloud_color_mode = color_mode;
        if let Some(manager) = self.scene_manager.as_mut() {
            let crop = self.inspector.crop_filter_from_state();
            let _ = manager.apply(Transaction::new(
                self.ids.next_transaction_id(),
                vec![SceneOperation::SetPointCloudRef {
                    entity_id,
                    point_cloud_ref: PointCloudRef {
                        asset_id,
                        color_mode,
                        crop_filter: Some(crop),
                    },
                }],
            ));
            self.point_cloud_cache.invalidate_all();
            self.splat_cache.invalidate_all();
            self.sync_runtime();
        }
    }

    fn apply_point_cloud_crop_filter(
        &mut self,
        entity_id: EntityId,
        asset_id: AssetId,
        crop: PointCloudCropBox,
    ) {
        if let Some(manager) = self.scene_manager.as_mut() {
            let _ = manager.apply(Transaction::new(
                self.ids.next_transaction_id(),
                vec![SceneOperation::SetPointCloudRef {
                    entity_id,
                    point_cloud_ref: PointCloudRef {
                        asset_id,
                        color_mode: self.inspector.point_cloud_color_mode,
                        crop_filter: Some(crop),
                    },
                }],
            ));
            self.point_cloud_cache.invalidate_all();
            self.splat_cache.invalidate_all();
            self.sync_runtime();
        }
    }

    fn send_copilot_message(&mut self) {
        let prompt = self.copilot_input.trim().to_string();
        if prompt.is_empty() {
            return;
        }
        self.copilot_messages.push(CopilotMessage {
            role: CopilotRole::User,
            text: prompt.clone(),
        });
        self.copilot_input.clear();

        let Some(manager) = self.scene_manager.as_ref() else {
            self.copilot_messages.push(CopilotMessage {
                role: CopilotRole::Assistant,
                text: "Open a project scene before using Copilot.".into(),
            });
            return;
        };

        let selection: Vec<_> = self.selection.primary().into_iter().collect();
        match self
            .copilot_engine
            .ask(&prompt, manager.scene(), &selection)
        {
            Ok(CopilotResponse::Answer(answer)) => {
                self.copilot_messages.push(CopilotMessage {
                    role: CopilotRole::Assistant,
                    text: answer,
                });
            }
            Ok(CopilotResponse::Proposal(proposal)) => {
                self.copilot_messages.push(CopilotMessage {
                    role: CopilotRole::Assistant,
                    text: format!("Proposal: {}", proposal.summary),
                });
                self.copilot_pending = Some(proposal);
                self.copilot_preview_manager = None;
            }
            Err(err) => {
                self.copilot_messages.push(CopilotMessage {
                    role: CopilotRole::Assistant,
                    text: format!("Copilot error: {err}"),
                });
            }
        }
    }

    fn preview_copilot_proposal(&mut self) {
        let Some(proposal) = self.copilot_pending.clone() else {
            return;
        };
        let Some(manager) = self.scene_manager.as_ref() else {
            return;
        };
        match CopilotEngine::preview(&proposal, manager.scene(), &mut self.ids) {
            Ok(preview) => {
                self.copilot_preview_manager = Some(preview);
                self.sync_runtime();
            }
            Err(err) => {
                self.copilot_messages.push(CopilotMessage {
                    role: CopilotRole::Assistant,
                    text: format!("Preview failed: {err}"),
                });
            }
        }
    }

    fn approve_copilot_proposal(&mut self) {
        let Some(proposal) = self.copilot_pending.take() else {
            return;
        };
        let Some(_manager) = self.scene_manager.as_mut() else {
            return;
        };
        let transaction = proposal.into_transaction(self.ids.next_transaction_id());
        if self.apply_scene_transaction(transaction) {
            self.copilot_preview_manager = None;
            if let Some(project) = self.project.as_ref() {
                let _ = project.save();
            }
            self.copilot_messages.push(CopilotMessage {
                role: CopilotRole::Assistant,
                text: "Applied approved Copilot transaction.".into(),
            });
        } else {
            self.copilot_messages.push(CopilotMessage {
                role: CopilotRole::Assistant,
                text: "Commit failed.".into(),
            });
        }
    }

    fn reject_copilot_proposal(&mut self) {
        self.copilot_pending = None;
        self.copilot_preview_manager = None;
        self.sync_runtime();
        self.copilot_messages.push(CopilotMessage {
            role: CopilotRole::Assistant,
            text: "Discarded Copilot proposal.".into(),
        });
    }

    fn apply_scene_transaction(&mut self, transaction: Transaction) -> bool {
        let Some(manager) = self.scene_manager.as_mut() else {
            return false;
        };
        match manager.apply(transaction.clone()) {
            Ok(()) => {
                self.sync_runtime();
                if !self.applying_remote_sync {
                    self.push_collab_transaction(transaction);
                    self.scene_dirty = true;
                }
                true
            }
            Err(err) => {
                tracing::warn!("scene transaction failed: {err}");
                false
            }
        }
    }

    fn persist_project(&mut self) {
        let Some(project) = self.project.as_mut() else {
            return;
        };
        let Some(manager) = self.scene_manager.as_ref() else {
            return;
        };
        *project.scene_mut() = manager.scene().clone();
        if let Err(err) = project.save() {
            tracing::warn!("project save failed: {err}");
            return;
        }
        if let Err(err) = project.clear_recovery() {
            tracing::warn!("recovery cleanup failed: {err}");
        }
        self.scene_dirty = false;
        self.last_autosave = Instant::now();
    }

    fn tick_autosave(&mut self) {
        if !self.scene_dirty {
            return;
        }
        if self.last_autosave.elapsed().as_secs() < 30 {
            return;
        }
        let Some(project) = self.project.as_mut() else {
            return;
        };
        let Some(manager) = self.scene_manager.as_ref() else {
            return;
        };
        *project.scene_mut() = manager.scene().clone();
        if let Err(err) = project.write_autosave() {
            tracing::warn!("autosave failed: {err}");
            return;
        }
        self.last_autosave = Instant::now();
    }

    fn restore_recovery_snapshot(&mut self) {
        let Some(project) = self.project.as_mut() else {
            return;
        };
        match project.recover_from_autosave() {
            Ok(_) => {
                let scene = project.scene().clone();
                self.scene_manager = Some(TransactionManager::new(scene.clone()));
                project_scene_to_ecs(&scene, &mut self.runtime);
                self.sync_runtime();
                self.recovery_snapshot = None;
                self.scene_dirty = false;
            }
            Err(err) => tracing::warn!("recovery restore failed: {err}"),
        }
    }

    fn dismiss_recovery_snapshot(&mut self) {
        if let Some(project) = self.project.as_ref() {
            let _ = project.clear_recovery();
        }
        self.recovery_snapshot = None;
    }

    fn refresh_copilot_engine(&mut self) {
        let key = self.copilot_api_key.trim();
        let api_key = if key.is_empty() {
            None
        } else {
            Some(key.to_string())
        };
        self.copilot_engine = CopilotEngine::new(Box::new(RemoteLlmProvider::new(
            RemoteLlmConfig::from_parts(api_key, None, None),
        )));
    }

    fn open_project_at(&mut self, path: PathBuf) {
        if !path.join("manifest.c3d.toml").is_file() {
            self.project_status = format!("Not a Create3D project: {}", path.display());
            return;
        }

        match Project::open(&path) {
            Ok(project) => {
                self.disconnect_collab();
                self.stop_robotics_bridge();
                self.copilot_preview_manager = None;
                self.copilot_pending = None;
                let scene = project.scene().clone();
                self.scene_manager = Some(TransactionManager::new(scene.clone()));
                self.project = Some(project);
                self.mesh_cache.invalidate_all();
                self.point_cloud_cache.invalidate_all();
                self.splat_cache.invalidate_all();
                self.selection.clear();
                self.gizmo_drag = None;
                self.scene_dirty = false;
                self.last_autosave = Instant::now();
                self.collab_comments.clear();
                self.collab_proposals.clear();
                if let Some(project) = self.project.as_ref() {
                    if let Ok(store) = CollabStore::load(project.root().join("collab")) {
                        self.collab_comments = store.comments();
                        self.collab_proposals = store.proposals();
                    }
                    self.recovery_snapshot =
                        project.recovery_is_newer().ok().and_then(|is_newer| {
                            if is_newer {
                                project.recovery_snapshot().ok().flatten()
                            } else {
                                None
                            }
                        });
                }
                project_scene_to_ecs(&scene, &mut self.runtime);
                self.refresh_robotics_targets();
                self.demo_entity = self.project.as_ref().and_then(|project| {
                    project.scene().entities().find_map(|entity| {
                        entity
                            .name
                            .as_ref()
                            .filter(|name| name.value == "Lamp" || name.value == "Cube")
                            .map(|_| entity.id)
                    })
                });
                if let Some(entity_id) = self.demo_entity {
                    self.selection.select(entity_id);
                }
                self.project_status = format!("Opened {}", path.display());
            }
            Err(err) => self.project_status = format!("Open failed: {err}"),
        }
    }

    fn export_project_glb(&mut self) {
        let Some(project) = self.project.as_ref() else {
            self.project_status = "No project loaded".into();
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("glTF Binary", &["glb"])
            .set_file_name("snapshot.glb")
            .save_file()
        else {
            return;
        };
        match project.export_gltf(&path) {
            Ok(report) => {
                self.project_status = format!(
                    "Exported {} meshes to {} ({} bytes)",
                    report.mesh_count,
                    path.display(),
                    report.byte_length
                );
            }
            Err(err) => self.project_status = format!("Export failed: {err}"),
        }
    }

    fn export_project_usd(&mut self) {
        let Some(project) = self.project.as_ref() else {
            self.project_status = "No project loaded".into();
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("USD ASCII", &["usda", "usd"])
            .set_file_name("snapshot.usda")
            .save_file()
        else {
            return;
        };
        match project.export_usd(&path) {
            Ok(report) => {
                self.project_status = format!(
                    "Exported {} meshes to {} ({} bytes)",
                    report.mesh_count,
                    path.display(),
                    report.byte_length
                );
            }
            Err(err) => self.project_status = format!("Export failed: {err}"),
        }
    }

    fn export_project_ply(&mut self) {
        let Some(project) = self.project.as_ref() else {
            self.project_status = "No project loaded".into();
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PLY Point Cloud", &["ply"])
            .set_file_name("snapshot.ply")
            .save_file()
        else {
            return;
        };
        match project.export_ply(&path) {
            Ok(report) => {
                self.project_status = format!(
                    "Exported {} points from {} entities to {} ({} bytes, binary PLY)",
                    report.point_count,
                    report.entity_count,
                    path.display(),
                    report.byte_length
                );
            }
            Err(err) => self.project_status = format!("Export failed: {err}"),
        }
    }

    fn export_project_gsplat(&mut self) {
        let Some(project) = self.project.as_ref() else {
            self.project_status = "No project loaded".into();
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("3DGS PLY", &["ply"])
            .set_file_name("snapshot-3dgs.ply")
            .save_file()
        else {
            return;
        };
        match project.export_gsplat_ply(&path) {
            Ok(report) => {
                self.project_status = format!(
                    "Exported {} splats from {} entities to {} ({} bytes)",
                    report.splat_count,
                    report.entity_count,
                    path.display(),
                    report.byte_length
                );
            }
            Err(err) => self.project_status = format!("Export failed: {err}"),
        }
    }

    fn push_collab_transaction(&mut self, transaction: Transaction) {
        let Some(client) = self.sync_client.as_ref() else {
            return;
        };
        if let Err(err) = client.push_transaction(transaction) {
            self.collab_status = format!("Sync push failed: {err}");
        }
    }

    fn connect_collab(&mut self) {
        match SyncClient::connect(SyncClientConfig {
            workspace_id: self.collab_workspace.clone(),
            user_name: self.collab_user_name.clone(),
            server_addr: self.collab_server_addr.clone(),
        }) {
            Ok(client) => {
                self.sync_client = Some(client);
                self.collab_status = "Connected to sync server".into();
            }
            Err(err) => self.collab_status = format!("Connect failed: {err}"),
        }
    }

    fn disconnect_collab(&mut self) {
        self.sync_client = None;
        self.remote_presence.clear();
        self.collab_status = "Disconnected from sync server".into();
    }

    fn update_collab_presence(&mut self) {
        let Some(client) = self.sync_client.as_ref() else {
            return;
        };
        let selected = self.selection.primary();
        let cursor = self
            .viewport_rect
            .contains(self.viewport_rect.center())
            .then_some([self.viewport_rect.center().x, self.viewport_rect.center().y]);
        let _ = client.update_presence(&self.collab_user_name, selected, cursor);
    }

    fn poll_collab(&mut self) {
        let Some(client) = self.sync_client.as_mut() else {
            return;
        };
        let events = client.poll_events();
        for event in events {
            match event {
                SyncEvent::Connected { .. } => {
                    self.collab_status = "Sync session active".into();
                    self.update_collab_presence();
                }
                SyncEvent::LogEntry(entry) => {
                    self.applying_remote_sync = true;
                    if let Some(manager) = self.scene_manager.as_mut() {
                        if let Err(err) = manager.apply(entry.transaction) {
                            tracing::warn!("remote sync apply failed: {err}");
                        } else {
                            self.sync_runtime();
                        }
                    }
                    self.applying_remote_sync = false;
                }
                SyncEvent::Presence(presence) => {
                    self.remote_presence
                        .retain(|peer| peer.client_id != presence.client_id);
                    self.remote_presence.push(presence);
                }
                SyncEvent::Comment(comment) => {
                    self.collab_comments
                        .retain(|existing| existing.id != comment.id);
                    self.collab_comments.push(comment);
                    self.persist_collab_store();
                }
                SyncEvent::CommentStatus { comment_id, status } => {
                    if let Some(comment) = self
                        .collab_comments
                        .iter_mut()
                        .find(|comment| comment.id == comment_id)
                    {
                        comment.status = status;
                        self.persist_collab_store();
                    }
                }
                SyncEvent::Proposal(proposal) => {
                    self.collab_proposals
                        .retain(|existing| existing.id != proposal.id);
                    self.collab_proposals.push(proposal);
                    self.persist_collab_store();
                }
                SyncEvent::ProposalStatus {
                    proposal_id,
                    status,
                } => {
                    if let Some(proposal) = self
                        .collab_proposals
                        .iter_mut()
                        .find(|proposal| proposal.id == proposal_id)
                    {
                        proposal.status = status;
                        self.persist_collab_store();
                    }
                }
                SyncEvent::Error(message) => self.collab_status = message,
                SyncEvent::Disconnected => {
                    self.collab_status = "Sync disconnected".into();
                    self.sync_client = None;
                }
            }
        }
    }

    fn persist_collab_store(&mut self) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        let mut store = CollabStore::default();
        for comment in &self.collab_comments {
            store.upsert_comment(comment.clone());
        }
        for proposal in &self.collab_proposals {
            store.upsert_proposal(proposal.clone());
        }
        let _ = store.save(project.root().join("collab"));
    }

    fn add_comment_for_selection(&mut self) {
        let Some(entity_id) = self.selection.primary() else {
            self.collab_status = "Select an entity to comment".into();
            return;
        };
        let text = self.collab_comment_input.trim();
        if text.is_empty() {
            return;
        }
        let comment = SceneComment::open(
            entity_id,
            &self.collab_user_name,
            text,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(0),
        );
        self.collab_comment_input.clear();
        if let Some(client) = self.sync_client.as_ref() {
            let _ = client.upsert_comment(comment.clone());
        }
        self.collab_comments.push(comment);
        self.persist_collab_store();
    }

    fn resolve_comment(&mut self, comment_id: c3d_collab_core::CommentId) {
        if let Some(client) = self.sync_client.as_ref() {
            let _ = client.set_comment_status(comment_id, CommentStatus::Resolved);
        }
        if let Some(comment) = self
            .collab_comments
            .iter_mut()
            .find(|comment| comment.id == comment_id)
        {
            comment.status = CommentStatus::Resolved;
            self.persist_collab_store();
        }
    }

    fn propose_copilot_branch(&mut self) {
        let Some(proposal) = self.copilot_pending.clone() else {
            return;
        };
        let client_id = self
            .sync_client
            .as_ref()
            .and_then(|client| client.client_id())
            .unwrap_or_default();
        let branch = BranchProposal::propose(
            proposal.summary.clone(),
            client_id,
            &self.collab_user_name,
            proposal.operations.clone(),
            Some(proposal.provenance.clone()),
        );
        if let Some(client) = self.sync_client.as_ref() {
            let _ = client.share_proposal(branch.clone());
        }
        self.collab_proposals.push(branch);
        self.persist_collab_store();
        self.collab_status = "Shared Copilot proposal as branch".into();
    }

    fn draw_collab_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Collaboration");
        if !self.collab_status.is_empty() {
            ui.label(&self.collab_status);
        }
        ui.horizontal(|ui| {
            ui.label("User");
            ui.text_edit_singleline(&mut self.collab_user_name);
            ui.label("Server");
            ui.text_edit_singleline(&mut self.collab_server_addr);
        });
        ui.horizontal(|ui| {
            if ui.button("Connect").clicked() {
                self.connect_collab();
            }
            if ui.button("Disconnect").clicked() {
                self.disconnect_collab();
            }
        });

        ui.separator();
        ui.label("Presence");
        if self.remote_presence.is_empty() {
            ui.label("No remote collaborators");
        } else {
            for peer in &self.remote_presence {
                let selection = peer
                    .selected_entity
                    .map(|entity| entity.to_string())
                    .unwrap_or_else(|| "none".into());
                ui.label(format!("{} -> selection {selection}", peer.user_name));
            }
        }

        ui.separator();
        ui.label("Comments");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.collab_comment_input);
            if ui.button("Add").clicked() {
                self.add_comment_for_selection();
            }
        });
        if let Some(entity_id) = self.selection.primary() {
            let comments: Vec<_> = self
                .collab_comments
                .iter()
                .filter(|comment| comment.entity_id == entity_id)
                .cloned()
                .collect();
            let mut resolve_ids = Vec::new();
            for comment in &comments {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "[{:?}] {}: {}",
                        comment.status, comment.author_name, comment.text
                    ));
                    if matches!(comment.status, CommentStatus::Open)
                        && ui.button("Resolve").clicked()
                    {
                        resolve_ids.push(comment.id);
                    }
                });
            }
            for comment_id in resolve_ids {
                self.resolve_comment(comment_id);
            }
        }

        ui.separator();
        ui.label("Branch Proposals");
        for proposal in &self.collab_proposals {
            ui.label(format!(
                "{} by {} [{:?}] ({} ops)",
                proposal.title,
                proposal.author_name,
                proposal.status,
                proposal.operations.len()
            ));
        }
    }

    fn draw_copilot_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Copilot");
        ui.label(
            "Ask about the scene or request edits. Write proposals require preview + approval.",
        );
        ui.horizontal(|ui| {
            ui.label("API key");
            if ui
                .add(egui::TextEdit::singleline(&mut self.copilot_api_key).password(true))
                .changed()
            {
                self.refresh_copilot_engine();
            }
        });
        ui.label(
            "Remote LLM via CREATE3D_COPILOT_API_KEY (optional CREATE3D_COPILOT_BASE_URL / CREATE3D_COPILOT_MODEL). Without a key, Copilot uses the local mock.",
        );
        egui::ScrollArea::vertical()
            .max_height(160.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for message in &self.copilot_messages {
                    let prefix = match message.role {
                        CopilotRole::User => "You",
                        CopilotRole::Assistant => "Copilot",
                    };
                    ui.label(format!("{prefix}: {}", message.text));
                }
            });

        let mut send = false;
        ui.horizontal(|ui| {
            let response = ui.text_edit_singleline(&mut self.copilot_input);
            if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                send = true;
            }
            if ui.button("Send").clicked() {
                send = true;
            }
        });
        if send {
            self.send_copilot_message();
        }

        if self.copilot_pending.is_some() {
            ui.separator();
            ui.label("Pending AI transaction");
            ui.horizontal(|ui| {
                if ui.button("Preview").clicked() {
                    self.preview_copilot_proposal();
                }
                if ui.button("Approve").clicked() {
                    self.approve_copilot_proposal();
                }
                if ui.button("Reject").clicked() {
                    self.reject_copilot_proposal();
                }
                if ui.button("Propose Branch").clicked() {
                    self.propose_copilot_branch();
                }
            });
            if self.copilot_preview_manager.is_some() {
                ui.label("Preview active in viewport.");
            }
        }
    }

    fn crop_selected_point_cloud(&mut self) {
        let Some(entity_id) = self.selection.primary() else {
            return;
        };
        let Some(project) = self.project.as_mut() else {
            return;
        };
        let Some(entity) = project.scene().get(entity_id) else {
            return;
        };
        let Some(point_cloud_ref) = entity.point_cloud_ref.clone() else {
            return;
        };
        let crop = self.inspector.crop_filter_from_state();
        match project.crop_point_cloud(
            point_cloud_ref.asset_id,
            crop,
            &mut self.ids,
            format!("{entity_id}-cropped"),
        ) {
            Ok(derived_id) => {
                if let Some(manager) = self.scene_manager.as_mut() {
                    let _ = manager.apply(Transaction::new(
                        self.ids.next_transaction_id(),
                        vec![SceneOperation::SetPointCloudRef {
                            entity_id,
                            point_cloud_ref: PointCloudRef {
                                asset_id: derived_id,
                                color_mode: point_cloud_ref.color_mode,
                                crop_filter: None,
                            },
                        }],
                    ));
                }
                self.point_cloud_cache.invalidate_all();
                self.splat_cache.invalidate_all();
                self.sync_runtime();
                if let Some(project) = self.project.as_ref() {
                    let _ = project.save();
                }
                tracing::info!("created derived cropped point cloud {derived_id}");
            }
            Err(err) => tracing::error!("crop point cloud failed: {err}"),
        }
    }

    fn redraw(&mut self) {
        let now = Instant::now();
        self.frame_ms = now.duration_since(self.last_frame).as_secs_f32() * 1000.0;
        self.last_frame = now;
        self.tick_robotics_bridge();
        self.poll_collab();
        self.update_collab_presence();
        self.tick_autosave();

        let window = self.window.clone().expect("window");
        let egui_ctx = self.egui_ctx.clone().expect("egui ctx");
        let mut egui_winit = self.egui_winit.take().expect("egui winit");
        let viewport_texture = self.viewport_texture.expect("viewport texture");
        let mut import_request = None;
        let mut next_viewport_extent = self.viewport_extent;

        if self.viewport_extent.width > 1 && self.viewport_extent.height > 1 {
            let gizmo_origin = self
                .selection
                .primary()
                .and_then(|entity_id| self.world_origin(entity_id));
            let backend = self.backend.as_mut().expect("backend");
            let viewport = self.viewport.as_mut().expect("viewport");
            let project = self.project.as_ref().expect("project");
            let _ = viewport.resize(backend, self.viewport_extent);
            let _ = viewport.prepare_gizmo(backend, gizmo_origin);
            let _ = viewport.render(
                backend,
                &self.camera,
                &mut self.runtime,
                project.assets(),
                &mut self.mesh_cache,
                &mut self.point_cloud_cache,
                &mut self.splat_cache,
                self.shading_mode,
            );
        }

        let raw_input = egui_winit.take_egui_input(&window);
        let full_output = egui_ctx.run(raw_input, |ctx| {
            import_request = self.handle_shortcuts(ctx);

            if self.recovery_snapshot.is_some() {
                egui::TopBottomPanel::top("recovery")
                    .exact_height(28.0)
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Recovery snapshot available from an unexpected shutdown.");
                            if ui.button("Restore").clicked() {
                                self.restore_recovery_snapshot();
                            }
                            if ui.button("Dismiss").clicked() {
                                self.dismiss_recovery_snapshot();
                            }
                        });
                    });
            }

            egui::SidePanel::left("hierarchy")
                .default_width(220.0)
                .resizable(true)
                .show(ctx, |ui| self.draw_hierarchy(ui));

            egui::SidePanel::left("robotics")
                .default_width(240.0)
                .resizable(true)
                .show(ctx, |ui| self.draw_robotics_panel(ui));

            egui::SidePanel::right("inspector")
                .default_width(260.0)
                .resizable(true)
                .show(ctx, |ui| self.draw_inspector(ui));

            egui::TopBottomPanel::bottom("collaboration")
                .default_height(180.0)
                .resizable(true)
                .show(ctx, |ui| self.draw_collab_panel(ui));

            egui::TopBottomPanel::bottom("copilot")
                .default_height(220.0)
                .resizable(true)
                .show(ctx, |ui| self.draw_copilot_panel(ui));

            egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                ui.label(format!(
                    "Create3D Beta {C3D_VERSION} | frame {:.1} ms | Ctrl+Shift+P palette",
                    self.frame_ms
                ));
                if self.scene_dirty {
                    ui.label("(unsaved changes)");
                }
                if !self.project_status.is_empty() {
                    ui.label(&self.project_status);
                }
                ui.separator();
                if ui.button("Open Project").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.open_project_at(path);
                    }
                }
                if ui.button("Save Project").clicked() {
                    self.persist_project();
                    self.project_status = "Project saved".into();
                }
                if ui.button("Export GLB").clicked() {
                    self.export_project_glb();
                }
                if ui.button("Export USD").clicked() {
                    self.export_project_usd();
                }
                if ui.button("Export PLY").clicked() {
                    self.export_project_ply();
                }
                if ui.button("Export 3DGS").clicked() {
                    self.export_project_gsplat();
                }
                ui.separator();
                if ui.button("Import GLB").clicked() {
                    import_request = rfd::FileDialog::new()
                        .add_filter("glTF", &["gltf", "glb"])
                        .pick_file()
                        .map(ImportRequest::Glb);
                }
                if ui.button("Import PLY").clicked() {
                    import_request = rfd::FileDialog::new()
                        .add_filter("PLY", &["ply"])
                        .pick_file()
                        .map(ImportRequest::AutoPly);
                }
                if ui.button("Import 3DGS").clicked() {
                    import_request = rfd::FileDialog::new()
                        .add_filter("3DGS PLY", &["ply"])
                        .pick_file()
                        .map(ImportRequest::GsplatPly);
                }
                if ui.button("Import URDF").clicked() {
                    import_request = rfd::FileDialog::new()
                        .add_filter("URDF", &["urdf", "xacro"])
                        .pick_file()
                        .map(ImportRequest::Urdf);
                }
                if ui.button("Undo").clicked() {
                    self.undo();
                }
                if ui.button("Redo").clicked() {
                    self.redo();
                }
                if ui.button("Focus").clicked() {
                    self.focus_selection();
                }
                if ui.button("Cube").clicked() {
                    self.create_primitive(PrimitiveKind::UnitCube, "Cube");
                }
                if ui.button("Plane").clicked() {
                    self.create_primitive(PrimitiveKind::Plane, "Plane");
                }
                if ui.button("Thumbnail").clicked() {
                    self.generate_selected_thumbnail();
                }
                ui.separator();
                egui::ComboBox::from_label("Shading")
                    .selected_text(self.shading_mode.label())
                    .show_ui(ui, |ui| {
                        for mode in ViewportShadingMode::all() {
                            ui.selectable_value(&mut self.shading_mode, mode, mode.label());
                        }
                    });
                let scene_manager = self.scene_manager.as_ref().expect("scene manager");
                ui.label(format!(
                    "Entities: {} | undo: {} | redo: {} | sel: {}",
                    scene_manager.scene().entity_count(),
                    scene_manager.can_undo(),
                    scene_manager.can_redo(),
                    self.selection.primary().map(|_| "yes").unwrap_or("no")
                ));
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Viewport");
                ui.label(
                    "Select: click | Gizmo: drag axis | Orbit: Alt+LMB | Pan: MMB | Zoom: wheel",
                );
                let available = ui.available_size();
                next_viewport_extent = Extent2D {
                    width: available.x.max(1.0) as u32,
                    height: available.y.max(1.0) as u32,
                };
                let aspect =
                    next_viewport_extent.width as f32 / next_viewport_extent.height.max(1) as f32;
                let viewport_size = Vec2::new(available.x, available.y);
                let response = ui.add(
                    egui::Image::new((viewport_texture, available))
                        .sense(egui::Sense::click_and_drag()),
                );
                self.viewport_rect = response.rect;
                self.handle_viewport_input(&response, response.rect, aspect, viewport_size);
            });
        });

        self.viewport_extent = next_viewport_extent;

        let backend = self.backend.as_mut().expect("backend");
        let egui_renderer = self.egui_renderer.as_mut().expect("egui renderer");

        egui_winit.handle_platform_output(&window, full_output.platform_output);

        let handles = backend.handles();
        for (id, image_delta) in &full_output.textures_delta.set {
            egui_renderer.update_texture(handles.device, handles.queue, *id, image_delta);
        }
        for id in &full_output.textures_delta.free {
            egui_renderer.free_texture(id);
        }

        let primitives = egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        let surface_frame = backend
            .acquire_surface_frame()
            .expect("acquire surface frame");
        let surface_view = surface_frame.view();

        let mut encoder = handles
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("egui-encoder"),
            });

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [window.inner_size().width, window.inner_size().height],
            pixels_per_point: window.scale_factor() as f32,
        };

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.10,
                        g: 0.10,
                        b: 0.10,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let mut render_pass = render_pass.forget_lifetime();
        egui_renderer.render(&mut render_pass, &primitives, &screen_descriptor);
        drop(render_pass);

        backend.submit([encoder.finish()]);
        surface_frame.present();

        if let Some(request) = import_request {
            self.dispatch_import(request);
        }

        self.egui_winit = Some(egui_winit);
    }
}

fn default_project_dir() -> PathBuf {
    std::env::temp_dir().join("create3d-desktop-project")
}

fn open_or_create_desktop_project(project_dir: &PathBuf) -> (Project, Option<EntityId>) {
    if project_dir.join("manifest.c3d.toml").is_file() {
        let project = Project::open(project_dir).expect("open desktop project");
        let demo_entity = project.scene().entities().find_map(|entity| {
            entity
                .name
                .as_ref()
                .filter(|name| name.value == "Lamp" || name.value == "Cube")
                .map(|_| entity.id)
        });
        return (project, demo_entity);
    }

    let project =
        Project::create_from_template(project_dir, "desktop-demo", ProjectTemplate::AiEditingDemo)
            .expect("create desktop project");
    let demo_entity = project.scene().entities().find_map(|entity| {
        entity
            .name
            .as_ref()
            .filter(|name| name.value == "Lamp")
            .map(|_| entity.id)
    });
    (project, demo_entity)
}
