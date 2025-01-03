use std::time::{Duration, Instant};

use anyhow::Context;
use cgmath::{Array, Deg, Point3, Quaternion, Rotation3, Vector3};
use log::{error, info, warn};
use wgpu::{util::DeviceExt, Device, PolygonMode, Queue, RenderPipeline, TextureFormat};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, WindowEvent},
    event_loop::EventLoopProxy,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{
    camera::{Camera, CameraController, CameraUniform, Projection},
    ecs::ecs::Res,
    engine_state::{EngineState, TextureWithView},
    noise::{NoiseGenerator, NoiseUniform},
    scene::Scene,
    ui::{
        console::ConsoleNode,
        renderer::{UiNode, UiRenderer},
        settings::SettingsNode,
        text::DebugOverlay,
    },
    voxel::{
        instance::{Instance, InstanceRaw, INSTANCE_DISPLACEMENT, NUM_INSTANCES_PER_ROW},
        light::LightUniform,
        model::{Material, Model},
        plane::Plane,
        texture,
        vertex::{ModelVertex, PrimitiveRenderer, Vertex},
    },
    CustomEvents,
};

pub struct State<'window> {
    scene: Scene,
    // engine_state: EngineState,
    wireframe_render_pipeline: RenderPipeline,
    render_pipeline: RenderPipeline,
    plane_renderer: PrimitiveRenderer,

    window: &'window Window,
    device: Device,
    queue: Queue,

    default_material: Res<Material>,

    settings: SettingsNode,
    show_settings: bool,
    ui_renderer: UiRenderer,

    pub console_node: ConsoleNode,
    show_console: bool,

    delta: Duration,
    surface: wgpu::Surface<'window>,

    size: winit::dpi::PhysicalSize<u32>,
    config: wgpu::SurfaceConfiguration,

    camera: Camera,
    projection: Projection,
    pub camera_controller: CameraController,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,

    depth_texture: Res<TextureWithView>,

    obj_model: Res<Model>,
    light_buffer: wgpu::Buffer,
    light_uniform: LightUniform,
    light_bind_group: wgpu::BindGroup,

    last_draw_call_ts: Instant,
    proxy: EventLoopProxy<CustomEvents>,

    noise_generator: NoiseGenerator,
    noise_material: Res<Material>,
    noise_uniform: NoiseUniform,
}
impl<'w> State<'w> {
    pub async fn new(
        window: &'w Window,
        proxy: EventLoopProxy<CustomEvents>,
    ) -> anyhow::Result<Self> {
        let size = window.inner_size();

        assert!(size.width != 0 && size.height != 0);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("Failed to get Adapter")?;

        info!("Using Device {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::POLYGON_MODE_LINE
                        | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                        | wgpu::Features::BGRA8UNORM_STORAGE,
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await?;

        let surface_capabilities = surface.get_capabilities(&adapter);
        info!("SURFACE CAPABILITES: {:?}", surface_capabilities);

        let swapchain_format = surface_capabilities.formats[0];
        // let format = TextureFormat::Rgba8UnormSrgb;

        let format = *surface_capabilities
            .formats
            .iter()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(&swapchain_format);

        let alpha_mode = surface_capabilities.alpha_modes[0];
        let present_mode = wgpu::PresentMode::AutoVsync;

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: Vec::new(),
        };

        info!("WGPU CONFIG {:?}", config);

        surface.configure(&device, &config);
        let mut engine_state = EngineState::default();

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let shader_code = tokio::fs::read_to_string("assets/shaders/shader.wgsl").await?;

        let default_material_sampler = engine_state.create_sampler("default".into(), &device)?;
        let default_material_texture = engine_state.create_texture(
            "default".into(),
            (1, 1),
            TextureFormat::Rgba8Unorm,
            &device,
        )?;
        engine_state.write_texture(
            &default_material_texture.texture,
            &queue,
            &[255, 255, 255, 255],
        );

        // Material::default_material(&device, &queue, &texture_bind_group_layout);

        let default_material = Material::new(
            &device,
            "default",
            default_material_texture,
            default_material_sampler.clone(),
            &texture_bind_group_layout,
        );

        let default_material = Res::new(default_material);
        let obj_model = Model::load_obj_model_from_file_path(
            "assets/models/plane_cube.obj".into(),
            &device,
            &queue,
            &texture_bind_group_layout,
            &mut engine_state,
        )
        .await?;

        let obj_model = Res::new(obj_model);

        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        };
        let camera = Camera::new(
            Point3::new(0.0, 5.0, 10.0),
            cgmath::Deg(-90.0).into(),
            cgmath::Deg(-20.0).into(),
        );
        let projection = Projection::new(
            config.width,
            config.height,
            cgmath::Deg(45.0).into(),
            0.1,
            100.0,
        );
        let camera_controller = CameraController::new(4.0, 0.4);
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        const SPACE_BETWEEN: f32 = 3.0;
        let instances: Vec<_> = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).flat_map(move |x| {
                    (0..NUM_INSTANCES_PER_ROW).map(move |y| {
                        let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                        let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                        let y = SPACE_BETWEEN * (y as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                        let position = cgmath::Vector3 { x, y, z } - INSTANCE_DISPLACEMENT;

                        let rotation = cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        );

                        let scale = cgmath::Vector3::new(1.0, 1.0, 1.0);

                        Instance {
                            position,
                            rotation,
                            scale,
                        }
                    })
                })
            })
            .collect();

        let instance_data: Vec<_> = instances.iter().map(Instance::to_raw).collect();
        info!("Instances {:?}", instance);
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let light_uniform = LightUniform::new(Vector3::new(1.0, 1.0, 1.0));

        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light VB"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("light_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: Some("light_bind_group"),
        });

        // let depth_texture =
        //     texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let depth_texture = engine_state.create_texture(
            "depth".into(),
            (config.width, config.height),
            TextureFormat::Depth32Float,
            &device,
        )?;

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_bind_group_layout,
                    &light_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let shader = device.create_shader_module(shader);

        let render_pipeline = create_render_pipeline(
            &device,
            &render_pipeline_layout,
            config.format,
            Some(texture::Texture::DEPTH_FORMAT),
            &[ModelVertex::desc(), InstanceRaw::desc()],
            &shader,
            "Render Pipeline",
            RenderPipeLineType::Default,
        );
        let wireframe_render_pipeline = create_render_pipeline(
            &device,
            &render_pipeline_layout,
            config.format,
            Some(texture::Texture::DEPTH_FORMAT),
            &[ModelVertex::desc(), InstanceRaw::desc()],
            &shader,
            "Wireframe Render Pipeline",
            RenderPipeLineType::Wireframe,
        );

        let last_draw_call_ts = Instant::now();

        let delta = Duration::ZERO;

        let ui_renderer =
            UiRenderer::new(&window, &device, config.format, config.width, config.height);
        let show_settings = false;
        let settings = SettingsNode::default();

        let plane_instance = Instance {
            position: Vector3::from_value(0.0),
            rotation: Quaternion::from_axis_angle(Vector3::unit_y(), Deg(0.0)),
            scale: Vector3::from_value(10.0),
        };

        let plane_renderer = PrimitiveRenderer::new::<Plane>(&device, vec![plane_instance]);

        let noise_uniform = NoiseUniform::new(rand::random(), 5.0, (0.0, 0.0), (1024, 1024));
        let noise_generator = NoiseGenerator::new(&device, noise_uniform).await?;
        window.set_cursor_visible(false);

        // let noise_material = Res::new(Material::new(
        //     &device,
        //     "Noise Material",
        //     texture::Texture::blank(&device, &queue, (1024, 1024), false),
        //     &texture_bind_group_layout,
        // ));

        let noise_texture = engine_state.create_texture(
            "noise".into(),
            (1024, 1024),
            TextureFormat::Rgba8Unorm,
            &device,
        )?;

        let noise_material = Material::new(
            &device,
            "noise",
            noise_texture,
            default_material_sampler,
            &texture_bind_group_layout,
        );

        let noise_material = Res::new(noise_material);
        let console_node = ConsoleNode::new(proxy.clone());
        let show_console = false;

        let scene = Scene::default();
        Ok(Self {
            scene,
            console_node,
            show_console,
            window,
            queue,
            device,
            noise_uniform,
            noise_material,
            noise_generator,
            proxy,
            plane_renderer,
            default_material,
            settings,
            ui_renderer,
            delta,
            wireframe_render_pipeline,
            projection,
            surface,
            config,
            size,
            render_pipeline,

            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,

            instances,
            instance_buffer,

            depth_texture,

            obj_model,

            light_buffer,
            light_uniform,

            light_bind_group,

            last_draw_call_ts,

            show_settings,
        })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        info!("Resize Event {:?}", new_size);
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.projection.resize(new_size.width, new_size.height);
            let depth_texture = TextureWithView::create(
                "depth".into(),
                (self.config.width, self.config.height),
                TextureFormat::Depth32Float,
                &self.device,
            );
            self.depth_texture = Res::new(depth_texture);
            self.ui_renderer
                .resize(self.config.width, self.config.height);
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, window_event: &WindowEvent) -> bool {
        let ui_consumed = self.ui_renderer.on_event(self.window, window_event);
        if ui_consumed {
            return true;
        }
        match window_event {
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let is_pressed = event.state == ElementState::Pressed;
                match event.physical_key {
                    PhysicalKey::Code(key_code) => match key_code {
                        KeyCode::Escape => {
                            if is_pressed {
                                self.show_settings = !self.show_settings;
                                if self.show_settings {
                                    self.show_cursor();
                                } else {
                                    self.hide_cursor();
                                }

                                if self.show_console {
                                    self.show_console = false;
                                    self.console_node.clear();
                                }
                            }

                            true
                        }
                        KeyCode::KeyR => {
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            let result =
                                rt.block_on(NoiseGenerator::new(&self.device, self.noise_uniform));

                            match result {
                                Ok(new) => {
                                    self.noise_generator = new;

                                    info!("Reloaded Noise Generator")
                                }
                                Err(err) => error!("{err}"),
                            };

                            true
                        }
                        KeyCode::Backquote => {
                            if is_pressed {
                                if !self.show_console {
                                    self.show_console = true;
                                    self.console_node.should_request_focus();
                                }
                            }
                            true
                        }
                        _ => {
                            let camera_consumed = self
                                .camera_controller
                                .process_keyboard(key_code, is_pressed);

                            camera_consumed || ui_consumed
                        }
                    },
                    PhysicalKey::Unidentified(key_code) => {
                        warn!("Unidentified KeyCode {:?}", key_code);
                        false
                    }
                }
            }

            _ => false,
        }
    }

    pub fn show_cursor(&self) {
        let size = self.size();
        self.window
            .set_cursor_position(PhysicalPosition::new(size.width / 2, size.height / 2))
            .unwrap();
        self.window.set_cursor_visible(true);
    }

    pub fn hide_cursor(&self) {
        self.window.set_cursor_visible(false);
    }

    pub fn update(&mut self, dt: Duration) {
        self.delta = dt;
        if !self.show_settings {
            self.camera_controller.update_camera(&mut self.camera, dt);
            let old_uniform = [self.camera_uniform];
            let old_slice: &[u8] = bytemuck::cast_slice(&old_uniform);
            self.camera_uniform
                .update_view_proj(&self.camera, &self.projection);
            let new_uniform = [self.camera_uniform];
            let new_slice: &[u8] = bytemuck::cast_slice(&new_uniform);
            if old_slice != new_slice {
                self.queue.write_buffer(&self.camera_buffer, 0, new_slice);
            }
        } else {
            if self.settings.show_noise {
                self.noise_generator
                    .update_uniform(&self.device, &self.queue, self.noise_uniform);
            }
        }

        if self.settings.full_screen {
            if self.window.fullscreen().is_none() {
                self.window
                    .set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            }
        } else {
            if self.window.fullscreen().is_some() {
                self.window.set_fullscreen(None);
            }
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if self.window.is_minimized().unwrap_or(false) {
            return Ok(());
        }
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            let mut material = &self.obj_model.materials[0];

            if self.settings.show_noise {
                material = &self.noise_material;
            }

            if self.settings.show_wireframe {
                render_pass.set_pipeline(&self.wireframe_render_pipeline);
            } else {
                render_pass.set_pipeline(&self.render_pipeline);
            }

            self.plane_renderer.draw_with_material(
                &material,
                &mut render_pass,
                &self.camera_bind_group,
                &self.light_bind_group,
            );
        }

        let mut should_read_noise_output = false;
        {
            self.ui_renderer.draw(
                &self.device,
                &self.queue,
                &self.window,
                &mut encoder,
                &view,
                |ui| {
                    if self.show_settings {
                        self.settings.add_ui(ui);
                        // self.noise_uniform.add_ui(ui);
                    }
                    if self.settings.show_fps {
                        DebugOverlay { dt: self.delta }.add_ui(ui);
                    }

                    if self.show_console {
                        self.console_node.add_ui(ui);
                    }
                },
            );

            if self.settings.save_noise_texture {
                self.settings.save_noise_texture = false;

                self.noise_generator.compute(&mut encoder);
                self.noise_generator
                    .copy_to_texture(&mut encoder, &self.noise_material.diffuse_texture.texture);

                should_read_noise_output = true;
            }
        }

        let _idx = self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        if should_read_noise_output {
            // self.noise_generator.async_read();
        }

        // let maintain = wgpu::MaintainBase::WaitForSubmissionIndex(idx);
        // loop {
        //     let result = self.render_target.device.poll(maintain.clone());
        //     match result {
        //         MaintainResult::SubmissionQueueEmpty => break,
        //         MaintainResult::Ok => continue,
        //     }
        // }

        // if should_read_noise_output {
        //     self.noise_generator.read_and_save_to_img();
        // }

        Ok(())
    }
    pub fn spawn(&mut self, mesh: MeshType) {
        match mesh {
            MeshType::Plane => {
                let plane_instance = Instance {
                    position: Vector3::from_value(0.0),
                    rotation: Quaternion::from_axis_angle(Vector3::unit_y(), Deg(0.0)),
                    scale: Vector3::from_value(1.0),
                };

                self.plane_renderer
                    .add_instance(plane_instance, &self.queue);
            }
        }
    }
}

pub fn save_tmp_image(size: (u32, u32), data: &[u8]) {
    info!("Saving Image /tmp/noise.png");
    image::save_buffer_with_format(
        "/tmp/noise.png",
        data,
        size.0,
        size.1,
        image::ExtendedColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .unwrap();
}

pub enum MeshType {
    Plane,
}

pub enum RenderPipeLineType {
    Default,
    Wireframe,
    Noise,
}

pub fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: &wgpu::ShaderModule,
    label: &str,
    ty: RenderPipeLineType,
) -> wgpu::RenderPipeline {
    let fragment_entry_point = match ty {
        RenderPipeLineType::Default => "fs_main",
        RenderPipeLineType::Wireframe => "fs_main_wf",
        RenderPipeLineType::Noise => "fs_main_noise",
    };
    let polygon_mode = if matches!(ty, RenderPipeLineType::Wireframe) {
        PolygonMode::Line
    } else {
        PolygonMode::Fill
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: "vs_main",
            buffers: vertex_layouts,
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: fragment_entry_point,
            targets: &[Some(color_format.into())],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}
