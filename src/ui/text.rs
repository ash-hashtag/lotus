use std::{fmt::Write, time::Duration};

pub struct DebugOverlayNode<'a> {
    color: [f32; 4],
    brush: wgpu_text::TextBrush<wgpu_text::glyph_brush::ab_glyph::FontRef<'a>>,
    delta: Duration,
    string_buf: String,
}

impl<'a> DebugOverlayNode<'a> {
    pub fn new(
        font_file_data: &'a [u8],
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        color: [f32; 4],
    ) -> Result<Self, wgpu_text::glyph_brush::ab_glyph::InvalidFont> {
        let brush = wgpu_text::BrushBuilder::using_font_bytes(font_file_data)?
            .build(device, width, height, format);
        let string_buf = String::new();
        let delta = Duration::ZERO;
        let color = color;

        Ok(Self {
            brush,
            string_buf,
            color,
            delta,
        })
    }

    pub fn update(&mut self, dt: Duration) {
        self.delta = dt;
    }

    pub fn queue(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.string_buf.clear();
        let fps = 1.0 / self.delta.as_secs_f32();
        unsafe {
            write!(&mut self.string_buf, "FPS: {fps}").unwrap_unchecked();

            let fps_text = self.string_buf.as_str();
            let sections = vec![wgpu_text::glyph_brush::Section::new()
                .add_text(wgpu_text::glyph_brush::Text::new(fps_text).with_color(self.color))];

            self.brush
                .queue(device, queue, sections.into())
                .unwrap_unchecked();
        };
    }

    pub fn draw<'b>(&'b self, ui_render_pass: &mut wgpu::RenderPass<'b>) {
        self.brush.draw(ui_render_pass);
    }
}
