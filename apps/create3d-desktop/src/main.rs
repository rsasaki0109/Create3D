//! Create3D desktop editor shell.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use c3d_asset_material::{MaterialAssetData, MaterialGraphData};
use c3d_core::logging::{init_logging, LoggingConfig};
use c3d_core::math::{Vec2, Vec3};
use c3d_core::{AssetId, EntityId, UlidGenerator};
use c3d_ecs::{project_scene_to_ecs, RuntimeWorld};
use c3d_editor_core::{CommandRegistry, SelectionState};
use c3d_mesh_authoring::PrimitiveKind;
use c3d_project::Project;
use c3d_rhi::Extent2D;
use c3d_rhi_wgpu::WgpuBackend;
use c3d_scene_ops::{SceneOperation, Transaction, TransactionManager};
use c3d_scene_schema::{
    Name, PointCloudColorMode, PointCloudCropBox, PointCloudRef, Transform, TransformOp,
};
use c3d_viewport::{
    gizmo_drag_delta, pick_entity, pick_gizmo_axis, GizmoDragState, MeshGpuCache, OrbitCamera,
    PointCloudGpuCache, ViewportRenderer, ViewportShadingMode,
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
    crop_min: [f32; 3],
    crop_max: [f32; 3],
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

        let mut project =
            Project::create(default_project_dir(), "desktop-demo").expect("create project");
        let mut scene_manager = TransactionManager::new(project.scene().clone());
        let demo_entity = self.ids.next_entity_id();
        scene_manager
            .apply(Transaction::new(
                self.ids.next_transaction_id(),
                vec![SceneOperation::CreateEntity {
                    entity_id: demo_entity,
                    parent: None,
                    name: Some(Name::new("DemoCube")),
                    transform: Transform::IDENTITY,
                    mesh_ref: None,
                    material_binding: None,
                    point_cloud_ref: None,
                }],
            ))
            .expect("create demo entity");
        *project.scene_mut() = scene_manager.scene().clone();
        project.save().expect("save project");
        project_scene_to_ecs(scene_manager.scene(), &mut self.runtime);
        self.selection.select(demo_entity);

        self.window = Some(window);
        self.backend = Some(backend);
        self.egui_ctx = Some(egui_ctx);
        self.egui_winit = Some(egui_winit);
        self.egui_renderer = Some(egui_renderer);
        self.viewport = Some(viewport);
        self.viewport_texture = Some(viewport_texture);
        self.scene_manager = Some(scene_manager);
        self.project = Some(project);
        self.demo_entity = Some(demo_entity);
        self.last_frame = Instant::now();
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
        if let Some(manager) = self.scene_manager.as_ref() {
            project_scene_to_ecs(manager.scene(), &mut self.runtime);
        }
        if let (Some(project), Some(manager)) = (self.project.as_mut(), self.scene_manager.as_ref())
        {
            *project.scene_mut() = manager.scene().clone();
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

    fn run_command(&mut self, command_id: &str) -> Option<PathBuf> {
        match command_id {
            "edit.undo" => {
                self.undo();
                None
            }
            "edit.redo" => {
                self.redo();
                None
            }
            "scene.import_glb" => rfd::FileDialog::new()
                .add_filter("glTF", &["gltf", "glb"])
                .pick_file(),
            "scene.import_ply" => rfd::FileDialog::new()
                .add_filter("PLY", &["ply"])
                .pick_file(),
            "pointcloud.crop_derived" => {
                self.crop_selected_point_cloud();
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
        self.sync_runtime();
        if let Some(project) = self.project.as_ref() {
            let _ = project.save();
        }
    }

    fn apply_transform_translate(&mut self, entity_id: EntityId, delta: Vec3) {
        if delta.length_squared() <= f32::EPSILON {
            return;
        }
        if let Some(manager) = self.scene_manager.as_mut() {
            let _ = manager.apply(Transaction::new(
                self.ids.next_transaction_id(),
                vec![SceneOperation::TransformOp {
                    entity_id,
                    op: TransformOp::Translate(delta),
                }],
            ));
            self.sync_runtime();
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
        self.inspector.point_cloud_asset_id = entity
            .point_cloud_ref
            .as_ref()
            .map(|point_cloud| point_cloud.asset_id);
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
            if let Some(manager) = self.scene_manager.as_mut() {
                let current = manager
                    .scene()
                    .get(entity_id)
                    .map(|entity| entity.transform)
                    .unwrap_or(Transform::IDENTITY);
                let mut next = current;
                next.translation = translation;
                let _ = manager.apply(Transaction::new(
                    self.ids.next_transaction_id(),
                    vec![SceneOperation::SetTransform {
                        entity_id,
                        transform: next,
                    }],
                ));
                self.sync_runtime();
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
            let selected = self.selection.is_selected(entity.id);
            if ui.selectable_label(selected, label).clicked() {
                self.selection.select(entity.id);
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) -> Option<PathBuf> {
        let mut import_path = None;
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
                import_path = self.run_command(command_id);
            }
        }

        import_path
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
            self.sync_runtime();
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

        let window = self.window.clone().expect("window");
        let egui_ctx = self.egui_ctx.clone().expect("egui ctx");
        let mut egui_winit = self.egui_winit.take().expect("egui winit");
        let viewport_texture = self.viewport_texture.expect("viewport texture");
        let mut import_path = None;
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
                self.shading_mode,
            );
        }

        let raw_input = egui_winit.take_egui_input(&window);
        let full_output = egui_ctx.run(raw_input, |ctx| {
            import_path = self.handle_shortcuts(ctx);

            egui::SidePanel::left("hierarchy")
                .default_width(220.0)
                .resizable(true)
                .show(ctx, |ui| self.draw_hierarchy(ui));

            egui::SidePanel::right("inspector")
                .default_width(260.0)
                .resizable(true)
                .show(ctx, |ui| self.draw_inspector(ui));

            egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                ui.label(format!(
                    "Create3D | frame {:.1} ms | Ctrl+Shift+P palette",
                    self.frame_ms
                ));
                ui.separator();
                if ui.button("Import GLB").clicked() {
                    import_path = rfd::FileDialog::new()
                        .add_filter("glTF", &["gltf", "glb"])
                        .pick_file();
                }
                if ui.button("Import PLY").clicked() {
                    import_path = rfd::FileDialog::new()
                        .add_filter("PLY", &["ply"])
                        .pick_file();
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

        if let Some(path) = import_path {
            if path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("ply"))
            {
                self.import_ply(path);
            } else {
                self.import_glb(path);
            }
        }

        self.egui_winit = Some(egui_winit);
    }
}

fn default_project_dir() -> PathBuf {
    std::env::temp_dir().join("create3d-desktop-project")
}
