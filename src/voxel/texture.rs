use std::path::Path;

use image::{EncodableLayout, GenericImageView};
use log::info;

#[derive(Debug)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub sampler: wgpu::Sampler,
    pub view: wgpu::TextureView,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            format: Self::DEPTH_FORMAT,
            dimension: wgpu::TextureDimension::D2,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(format!("{label}_sampler").as_str()),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }

    pub async fn from_file_path(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        file_path: &Path,
        is_normal_map: bool,
    ) -> anyhow::Result<Self> {
        info!("Loading Texture from {:?}", file_path);
        let data = tokio::fs::read(file_path).await?;
        Ok(Self::from_bytes(
            device,
            queue,
            &data,
            &format!("{:?}_texture", file_path),
            is_normal_map,
        )?)
    }

    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
        is_normal_map: bool,
    ) -> image::ImageResult<Self> {
        let img = image::load_from_memory(bytes)?;
        Ok(Self::from_image(device, queue, &img, label, is_normal_map))
    }

    pub fn create_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: (u32, u32),
        rgba: &[u8],
        format: wgpu::TextureFormat,
        label: &str,
    ) -> Self {
        let (width, height) = size;
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }

    pub fn default_normal_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let rgba = [128, 128, 255, 255];

        return Self::create_texture(
            device,
            queue,
            (1, 1),
            &rgba,
            wgpu::TextureFormat::Rgba8Unorm,
            "Default Normal Texture",
        );
    }

    pub fn default_diffuse_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let rgba = [128, 128, 255, 255];

        return Self::create_texture(
            device,
            queue,
            (1, 1),
            &rgba,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "Default Diffuse Texture",
        );
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: &str,
        is_normal_map: bool,
    ) -> Self {
        let format = if is_normal_map {
            wgpu::TextureFormat::Rgba8Unorm
        } else {
            wgpu::TextureFormat::Rgba8UnormSrgb
        };

        Self::create_texture(
            device,
            queue,
            img.dimensions(),
            img.to_rgba8().as_bytes(),
            format,
            label,
        )
    }
}
