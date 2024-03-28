use std::{
    path::Path,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context};
use cgmath::{Point3, Rotation3, Vector3};
use log::{info, warn};
use wgpu::util::DeviceExt;
use winit::{
    dpi::{LogicalPosition, PhysicalPosition},
    event::{ElementState, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{
    camera::{Camera, CameraController, CameraUniform, Projection},
    pipelines::wireframe::{DrawWireframe, WireframeRenderPipeline},
    ui::{
        renderer::{UiNode, UiRenderer},
        settings::SettingsNode,
        text::DebugOverlay,
    },
    voxel::{
        instance::{Instance, InstanceRaw, INSTANCE_DISPLACEMENT, NUM_INSTANCES_PER_ROW},
        light::LightUniform,
        model::{DrawLight, DrawModel, Material, Model},
        texture,
        vertex::{ModelVertex, Vertex, INDICES, VERTICES},
    },
};

pub struct RenderTarget<'a> {
    pub window: &'a Window,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

pub struct State<'window> {
    render_target: RenderTarget<'window>,
    default_material: Material,

    settings: SettingsNode,
    show_settings: bool,
    ui_renderer: UiRenderer,
    delta: Duration,
    surface: wgpu::Surface<'window>,

    size: winit::dpi::PhysicalSize<u32>,
    config: wgpu::SurfaceConfiguration,

    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,

    index_buffer: wgpu::Buffer,
    num_indices: u32,

    camera: Camera,
    projection: Projection,
    pub camera_controller: CameraController,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,

    depth_texture: texture::Texture,

    obj_model: Model,
    light_buffer: wgpu::Buffer,
    light_uniform: LightUniform,
    light_bind_group: wgpu::BindGroup,
    light_render_pipeline: wgpu::RenderPipeline,

    last_draw_call_ts: Instant,

    debug_material: Material,

    wireframe_render_pipeline: Option<WireframeRenderPipeline>,
}
impl<'w> State<'w> {
    pub async fn new(window: &'w Window) -> anyhow::Result<Self> {
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
            .with_context(|| anyhow!("Failed to get Adapter"))?;

        info!("Using Device {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::POLYGON_MODE_LINE,
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await?;

        let surface_capabilities = surface.get_capabilities(&adapter);
        info!("SURFACE CAPABILITES: {:?}", surface_capabilities);

        let swapchain_format = surface_capabilities.formats[0];
        let format = *surface_capabilities
            .formats
            .iter()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(&swapchain_format);
        // let present_mode = surface_capabilities.present_modes[0];
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let (shader_code, light_shader_code) = futures::future::try_join(
            tokio::fs::read_to_string("assets/shaders/shader.wgsl"),
            tokio::fs::read_to_string("assets/shaders/light.wgsl"),
        )
        .await?;

        let obj_model = Model::load_obj_model_from_file_path(
            "assets/models/plane_cube.obj".into(),
            &device,
            &queue,
            &texture_bind_group_layout,
        )
        .await?;

        let default_material =
            Material::default_material(&device, &queue, &texture_bind_group_layout);

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

                        Instance { position, rotation }
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

        let light_uniform =
            LightUniform::new(Vector3::new(2.0, 2.0, 2.0), Vector3::new(1.0, 1.0, 1.0));

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

        let wireframe_render_pipeline = Some(
            WireframeRenderPipeline::new(&device, &camera_bind_group_layout, config.format).await?,
        );

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: Some("light_bind_group"),
        });

        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Light Shader"),
                source: wgpu::ShaderSource::Wgsl(light_shader_code.into()),
            };

            create_render_pipeline(
                &device,
                &layout,
                config.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[ModelVertex::desc()],
                shader,
                "Light Render Pipeline",
            )
        };

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let num_vertices = VERTICES.len() as u32;

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

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

        let render_pipeline = create_render_pipeline(
            &device,
            &render_pipeline_layout,
            config.format,
            Some(texture::Texture::DEPTH_FORMAT),
            &[ModelVertex::desc(), InstanceRaw::desc()],
            shader,
            "Render Pipeline",
        );

        let last_draw_call_ts = Instant::now();

        let debug_material = {
            let diffuse_texture = texture::Texture::from_file_path(
                &device,
                &queue,
                &Path::new("assets/models/cobble-diffuse-resized.png"),
                false,
            )
            .await?;
            let normal_texture = texture::Texture::from_file_path(
                &device,
                &queue,
                &Path::new("assets/models/cobble-normal-resized.png"),
                true,
            )
            .await?;

            Material::new(
                &device,
                "debug-material",
                diffuse_texture,
                normal_texture,
                &texture_bind_group_layout,
            )
        };

        let delta = Duration::ZERO;

        let ui_renderer =
            UiRenderer::new(&window, &device, config.format, config.width, config.height);
        let show_settings = false;
        let settings = SettingsNode::default();

        window.set_cursor_visible(false);
        let render_target = RenderTarget {
            window,
            device,
            queue,
        };

        Ok(Self {
            default_material,
            render_target,
            settings,
            ui_renderer,
            delta,
            wireframe_render_pipeline,
            projection,
            debug_material,
            surface,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            num_vertices,
            index_buffer,
            num_indices,

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
            light_render_pipeline,

            light_bind_group,

            last_draw_call_ts,

            show_settings,
        })
    }

    pub fn window(&self) -> &Window {
        &self.render_target.window
    }

    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;

            self.projection.resize(new_size.width, new_size.height);

            self.depth_texture = texture::Texture::create_depth_texture(
                &self.render_target.device,
                &self.config,
                "depth_texture",
            );

            self.ui_renderer.resize(new_size.width, new_size.height);
            self.surface
                .configure(&self.render_target.device, &self.config);
        }
    }

    pub fn input(&mut self, window_event: &WindowEvent) -> bool {
        let ui_consumed = self
            .ui_renderer
            .on_event(self.render_target.window, window_event);
        match window_event {
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            // WindowEvent::MouseInput { state, .. } => {
            //     false
            // }
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

        self.render_target
            .window
            .set_cursor_position(PhysicalPosition::new(size.width / 2, size.height / 2))
            .unwrap();
        self.render_target.window.set_cursor_visible(true);
    }

    pub fn hide_cursor(&self) {
        self.render_target.window.set_cursor_visible(false);
    }

    pub fn update(&mut self, dt: Duration) {
        self.delta = dt;

        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
        self.render_target.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        if self.settings.full_screen {
            if self.render_target.window.fullscreen().is_none() {
                self.render_target
                    .window
                    .set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            }
        } else {
            if self.render_target.window.fullscreen().is_some() {
                self.render_target.window.set_fullscreen(None);
            }
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if self.render_target.window.is_minimized().unwrap_or(false) {
            return Ok(());
        }
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.render_target
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

            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            let instances = 0..self.instances.len() as u32;

            if self.settings.show_wireframe {
                if let Some(wireframe) = &self.wireframe_render_pipeline {
                    render_pass.set_pipeline(&wireframe.render_pipeline);
                    render_pass.draw_wireframe_instanced(
                        &self.obj_model,
                        instances,
                        &self.camera_bind_group,
                    );
                }
            } else {
                render_pass.set_pipeline(&self.light_render_pipeline);
                render_pass.draw_light_model(
                    &self.obj_model,
                    &self.camera_bind_group,
                    &self.light_bind_group,
                );
                render_pass.set_pipeline(&self.render_pipeline);
                // render_pass.draw_model_instanced(
                //     &self.obj_model,
                //     instances,
                //     &self.camera_bind_group,
                //     &self.light_bind_group,
                // );
                render_pass.draw_model_instanced_with_material(
                    &self.obj_model,
                    &self.default_material,
                    instances,
                    &self.camera_bind_group,
                    &self.light_bind_group,
                );
            }
        }

        {
            self.ui_renderer
                .draw(&self.render_target, &mut encoder, &view, |ui| {
                    if self.settings.show_fps {
                        DebugOverlay { dt: self.delta }.add_ui(ui);
                    }
                    if self.show_settings {
                        self.settings.add_ui(ui);
                    }
                });
        }

        self.render_target
            .queue
            .submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
    label: &str,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(shader);
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: vertex_layouts,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(color_format.into())],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
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
