pub struct UiRenderer {
    renderer: egui_wgpu::Renderer,
    screen_discriptor: egui_wgpu::ScreenDescriptor,
    paint_jobs: Vec<epaint::ClippedPrimitive>,
    ctx: egui::Context,
}

impl UiRenderer {
    pub fn context(&self) -> &egui::Context {
        &self.ctx
    }

    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let renderer = egui_wgpu::Renderer::new(device, color_format, None, 1);
        let screen_discriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: 1.0,
        };

        let ctx = egui::Context::default();

        let paint_jobs = vec![];

        Self {
            renderer,
            screen_discriptor,
            paint_jobs,
            ctx,
        }
    }

    pub fn draw(
        &mut self,
        full_output: egui::FullOutput,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let paint_jobs = self
            .ctx
            .tessellate(full_output.shapes, self.screen_discriptor.pixels_per_point);
        self.paint_jobs = paint_jobs;
        // update textures too

        for (id, texture) in full_output.textures_delta.set {
            self.renderer.update_texture(device, queue, id, &texture);
        }

        self.renderer.update_buffers(
            device,
            queue,
            encoder,
            &self.paint_jobs,
            &self.screen_discriptor,
        );
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui Render Pass"),
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
            .render(&mut render_pass, &self.paint_jobs, &self.screen_discriptor);
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.screen_discriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: 1.0,
        };
    }
}
pub trait UiNode {
    fn add_ui(&self, ui: &mut egui::Ui);
}
