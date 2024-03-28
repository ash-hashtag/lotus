use crate::state::RenderTarget;

pub struct UiRenderer {
    renderer: egui_wgpu::Renderer,
    screen_discriptor: egui_wgpu::ScreenDescriptor,
    state: egui_winit::State,
}

impl UiRenderer {
    pub fn context(&self) -> &egui::Context {
        self.state.egui_ctx()
    }

    pub fn new(
        window: &winit::window::Window,
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let ctx = egui::Context::default();
        let viewport_id = ctx.viewport_id();

        let pixels_per_point = egui_winit::pixels_per_point(&ctx, window);
        let state = egui_winit::State::new(ctx, viewport_id, window, None, None);
        let renderer = egui_wgpu::Renderer::new(device, color_format, None, 1);

        let screen_discriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point,
        };

        Self {
            renderer,
            screen_discriptor,
            state,
        }
    }

    fn render(
        &mut self,
        full_output: egui::FullOutput,
        render_target: &RenderTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let device = &render_target.device;
        let queue = &render_target.queue;
        let paint_jobs = self
            .context()
            .tessellate(full_output.shapes, self.screen_discriptor.pixels_per_point);

        for (id, texture) in full_output.textures_delta.set {
            self.renderer.update_texture(device, queue, id, &texture);
        }

        self.renderer
            .update_buffers(device, queue, encoder, &paint_jobs, &self.screen_discriptor);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Ui Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        self.renderer
            .render(&mut render_pass, &paint_jobs, &self.screen_discriptor);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.screen_discriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: 1.0,
        };
    }

    fn input(&mut self, window: &winit::window::Window) -> egui::RawInput {
        self.state.take_egui_input(window)
    }

    pub fn on_event(
        &mut self,
        window: &winit::window::Window,
        event: &winit::event::WindowEvent,
    ) -> bool {
        self.state.on_window_event(window, event).consumed
    }

    pub fn draw(
        &mut self,
        render_target: &RenderTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        run_ui: impl FnOnce(&mut egui::Ui),
    ) {
        let input = self.input(render_target.window);
        let full_output = self.context().run(input, |ctx| {
            let frame = egui::Frame::default().fill(egui::Color32::TRANSPARENT);
            let panel = egui::CentralPanel::default().frame(frame);

            panel.show(ctx, run_ui);
        });

        self.render(full_output, render_target, encoder, view);
    }
}
pub trait UiNode {
    fn add_ui(&mut self, ui: &mut egui::Ui);
}
