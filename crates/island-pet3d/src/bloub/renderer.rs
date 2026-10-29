use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use slint::wgpu_30::wgpu;
use wgpu::util::DeviceExt;

use super::decor::{ArcRender, Dot, NotifRender};
use super::face::eye_transform;
use super::math::TAU;
use super::shape::Silhouette;
use super::{BloubFrame, BloubPalette};

const SUPERSAMPLE: u32 = 2;
const TEXTURE_SIZE: u32 = 192;
const LATITUDE_SEGMENTS: usize = 32;
const LONGITUDE_SEGMENTS: usize = 64;
const BODY_DEPTH: f32 = 0.55;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct BodyUniform {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
    body_color: [f32; 4],
    eye_color: [f32; 4],
    params: [f32; 4],
    eye_centers: [[f32; 4]; 2],
    eye_inv: [[f32; 4]; 2],
    eye_extra: [[f32; 4]; 2],
    eye_style: [[f32; 4]; 2],
    notif: [f32; 4],
    light_dirs: [[f32; 4]; 3],
    light_colors: [[f32; 4]; 3],
    ambient: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct BodyVertex {
    position: [f32; 3],
    normal: [f32; 3],
}

impl BodyVertex {
    const ATTRS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &Self::ATTRS,
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct OverlayVertex {
    position: [f32; 3],
    color: [f32; 4],
}

impl OverlayVertex {
    const ATTRS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];

    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &Self::ATTRS,
    };
}

struct DynBuffer {
    buffer: wgpu::Buffer,
    capacity: u64,
    usage: wgpu::BufferUsages,
    label: &'static str,
}

impl DynBuffer {
    fn new(device: &wgpu::Device, label: &'static str, usage: wgpu::BufferUsages) -> Self {
        let capacity = 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: capacity,
            usage,
            mapped_at_creation: false,
        });
        Self {
            buffer,
            capacity,
            usage,
            label,
        }
    }

    fn write<T: Pod>(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[T]) {
        if data.is_empty() {
            return;
        }
        let bytes: &[u8] = bytemuck::cast_slice(data);
        let needed = bytes.len() as u64;
        if needed > self.capacity {
            self.capacity = needed.next_power_of_two().max(256);
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: self.capacity,
                usage: self.usage,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.buffer, 0, bytes);
    }
}

pub struct BloubRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,

    body_pipeline: wgpu::RenderPipeline,
    overlay_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,

    body_bind_group: wgpu::BindGroup,
    composite_bind_group: wgpu::BindGroup,
    body_uniform: wgpu::Buffer,

    body_vertices: DynBuffer,
    body_indices: DynBuffer,
    back_vertices: DynBuffer,
    front_vertices: DynBuffer,

    body_index_count: u32,
    back_vertex_count: u32,
    front_vertex_count: u32,

    _supersample_texture: wgpu::Texture,
    supersample_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    final_texture: wgpu::Texture,
    final_view: wgpu::TextureView,
}

impl BloubRenderer {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Result<Self, String> {
        let supersample_size = TEXTURE_SIZE * SUPERSAMPLE;
        let supersample_texture = create_color_texture(
            &device,
            "bloub-supersample",
            supersample_size,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let supersample_view =
            supersample_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let final_texture = create_color_texture(
            &device,
            "bloub-final",
            TEXTURE_SIZE,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let final_view = final_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bloub-depth"),
            size: wgpu::Extent3d {
                width: supersample_size,
                height: supersample_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let body_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("bloub-body-bind-group-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let composite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("bloub-composite-bind-group-layout"),
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
            });

        let body_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloub-body-pipeline-layout"),
            bind_group_layouts: &[Some(&body_bind_group_layout)],
            immediate_size: 0,
        });
        let overlay_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("bloub-overlay-pipeline-layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });
        let composite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("bloub-composite-pipeline-layout"),
                bind_group_layouts: &[Some(&composite_bind_group_layout)],
                immediate_size: 0,
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloub-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let body_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloub-body-pipeline"),
            layout: Some(&body_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_body"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(BodyVertex::LAYOUT)],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_body"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let overlay_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloub-overlay-pipeline"),
            layout: Some(&overlay_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_overlay"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(OverlayVertex::LAYOUT)],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_overlay"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloub-composite-pipeline"),
            layout: Some(&composite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_composite"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_composite"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let body_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bloub-body-uniform"),
            contents: bytemuck::bytes_of(&BodyUniform::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let body_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloub-body-bind-group"),
            layout: &body_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: body_uniform.as_entire_binding(),
            }],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloub-composite-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloub-composite-bind-group"),
            layout: &composite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&supersample_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Ok(Self {
            device: device.clone(),
            queue,
            body_pipeline,
            overlay_pipeline,
            composite_pipeline,
            body_bind_group,
            composite_bind_group,
            body_uniform,
            body_vertices: DynBuffer::new(
                &device,
                "bloub-body-vertices",
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            ),
            body_indices: DynBuffer::new(
                &device,
                "bloub-body-indices",
                wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            ),
            back_vertices: DynBuffer::new(
                &device,
                "bloub-back-vertices",
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            ),
            front_vertices: DynBuffer::new(
                &device,
                "bloub-front-vertices",
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            ),
            body_index_count: 0,
            back_vertex_count: 0,
            front_vertex_count: 0,
            _supersample_texture: supersample_texture,
            supersample_view,
            depth_view,
            final_texture,
            final_view,
        })
    }

    pub fn image(&self) -> Result<slint::Image, slint::wgpu_30::TextureImportError> {
        slint::Image::try_from(self.final_texture.clone())
    }

    pub fn image_size(&self) -> u32 {
        TEXTURE_SIZE
    }

    pub fn read_rgba(&self) -> Result<Vec<u8>, String> {
        let size = TEXTURE_SIZE;
        let bytes_per_row = size * 4;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloub-readback"),
            size: u64::from(bytes_per_row * size),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bloub-readback-encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.final_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(size),
                },
            },
            wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|err| format!("Bloub readback poll failed: {err}"))?;
        receiver
            .recv()
            .map_err(|err| format!("Bloub readback callback failed: {err}"))?
            .map_err(|err| format!("Bloub readback map failed: {err}"))?;
        let data = slice
            .get_mapped_range()
            .map_err(|err| format!("Bloub readback range failed: {err}"))?
            .to_vec();
        buffer.unmap();
        Ok(data)
    }

    pub fn update(&mut self, frame: &BloubFrame, palette: BloubPalette) {
        let extent = frame.extent.max(1.08);
        let (vertices, indices) = build_body(&frame.silhouette);
        self.body_vertices
            .write(&self.device, &self.queue, &vertices);
        self.body_indices.write(&self.device, &self.queue, &indices);
        self.body_index_count = indices.len() as u32;

        let uniform = build_uniform(frame, palette, extent);
        self.queue
            .write_buffer(&self.body_uniform, 0, bytemuck::bytes_of(&uniform));

        let (back, front) = build_overlay(frame, palette, extent);
        self.back_vertices.write(&self.device, &self.queue, &back);
        self.front_vertices.write(&self.device, &self.queue, &front);
        self.back_vertex_count = back.len() as u32;
        self.front_vertex_count = front.len() as u32;
    }

    pub fn render(&self) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bloub-encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloub-scene-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.supersample_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if self.back_vertex_count > 0 {
                pass.set_pipeline(&self.overlay_pipeline);
                pass.set_vertex_buffer(0, self.back_vertices.buffer.slice(..));
                pass.draw(0..self.back_vertex_count, 0..1);
            }

            pass.set_pipeline(&self.body_pipeline);
            pass.set_bind_group(0, &self.body_bind_group, &[]);
            pass.set_vertex_buffer(0, self.body_vertices.buffer.slice(..));
            pass.set_index_buffer(
                self.body_indices.buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.draw_indexed(0..self.body_index_count, 0, 0..1);

            if self.front_vertex_count > 0 {
                pass.set_pipeline(&self.overlay_pipeline);
                pass.set_vertex_buffer(0, self.front_vertices.buffer.slice(..));
                pass.draw(0..self.front_vertex_count, 0..1);
            }
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloub-composite-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.final_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.composite_pipeline);
            pass.set_bind_group(0, &self.composite_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
    }
}

fn create_color_texture(
    device: &wgpu::Device,
    label: &str,
    size: u32,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

fn build_body(sil: &Silhouette) -> (Vec<BodyVertex>, Vec<u32>) {
    let mut positions = Vec::with_capacity((LATITUDE_SEGMENTS + 1) * LONGITUDE_SEGMENTS);
    let cr = sil.rot.cos();
    let sr = sil.rot.sin();
    let (flip_sin, flip_cos) = sil.flip.sin_cos();
    let depth = BODY_DEPTH * (sil.sx + sil.sy) * 0.5;
    for lat in 0..=LATITUDE_SEGMENTS {
        let phi = -std::f32::consts::FRAC_PI_2
            + std::f32::consts::PI * lat as f32 / LATITUDE_SEGMENTS as f32;
        let cp = phi.cos();
        let sp = phi.sin();
        for lon in 0..LONGITUDE_SEGMENTS {
            let theta = lon as f32 / LONGITUDE_SEGMENTS as f32 * TAU;
            let r = sil.radii[lon];
            let x0 = r * theta.cos();
            let y0 = r * theta.sin();
            let rx = x0 * cr - y0 * sr;
            let ry = x0 * sr + y0 * cr;
            let x = rx * sil.sx * cp + sil.cx;
            let y0 = -(ry * sil.sy * cp + sil.cy);
            let z0 = depth * sp;
            let y = y0 * flip_cos - z0 * flip_sin;
            let z = y0 * flip_sin + z0 * flip_cos;
            positions.push([x, y, z]);
        }
    }

    let mut indices = Vec::with_capacity(LATITUDE_SEGMENTS * LONGITUDE_SEGMENTS * 6);
    for lat in 0..LATITUDE_SEGMENTS {
        for lon in 0..LONGITUDE_SEGMENTS {
            let next = (lon + 1) % LONGITUDE_SEGMENTS;
            let a = (lat * LONGITUDE_SEGMENTS + lon) as u32;
            let b = (lat * LONGITUDE_SEGMENTS + next) as u32;
            let c = ((lat + 1) * LONGITUDE_SEGMENTS + lon) as u32;
            let d = ((lat + 1) * LONGITUDE_SEGMENTS + next) as u32;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }

    let mut normals = vec![Vec3::ZERO; positions.len()];
    for tri in indices.chunks_exact(3) {
        let p0 = Vec3::from_array(positions[tri[0] as usize]);
        let p1 = Vec3::from_array(positions[tri[1] as usize]);
        let p2 = Vec3::from_array(positions[tri[2] as usize]);
        let n = (p1 - p0).cross(p2 - p0);
        if n.length_squared() > 1e-12 {
            normals[tri[0] as usize] += n;
            normals[tri[1] as usize] += n;
            normals[tri[2] as usize] += n;
        }
    }

    let vertices = positions
        .into_iter()
        .zip(normals)
        .map(|(position, normal)| {
            let normal = if normal.length_squared() > 1e-12 {
                normal.normalize()
            } else {
                Vec3::new(0.0, 0.0, if position[2] >= 0.0 { 1.0 } else { -1.0 })
            };
            BodyVertex {
                position,
                normal: normal.to_array(),
            }
        })
        .collect();
    (vertices, indices)
}

fn build_uniform(frame: &BloubFrame, palette: BloubPalette, extent: f32) -> BodyUniform {
    let camera_pos = Vec3::new(0.0, 0.0, 3.0);
    let view = Mat4::look_at_rh(camera_pos, Vec3::ZERO, Vec3::Y);
    let projection = Mat4::orthographic_rh(-extent, extent, -extent, extent, 0.1, 10.0);
    let view_proj = projection * view;

    let mut uniform = BodyUniform::zeroed();
    uniform.view_proj = view_proj.to_cols_array_2d();
    uniform.camera_pos = camera_pos.extend(1.0).to_array();
    uniform.body_color = palette.body;
    uniform.eye_color = palette.eye;
    uniform.params = [frame.body_alpha, frame.eye_alpha, 0.0, 0.0];
    uniform.light_dirs = [
        Vec3::new(-0.45, 0.80, 0.65)
            .normalize()
            .extend(0.0)
            .to_array(),
        Vec3::new(0.75, 0.20, 0.50)
            .normalize()
            .extend(0.0)
            .to_array(),
        Vec3::new(0.20, 0.55, -0.80)
            .normalize()
            .extend(0.0)
            .to_array(),
    ];
    uniform.light_colors = [
        [1.35, 1.28, 1.18, 0.0],
        [0.42, 0.52, 0.85, 0.0],
        [0.85, 0.72, 0.55, 0.0],
    ];
    uniform.ambient = [0.18, 0.19, 0.24, 0.0];

    for (index, eye) in frame.eyes.iter().enumerate() {
        let (center, inv, hw, hh, open, visible) = eye_transform(&frame.silhouette, eye);
        uniform.eye_centers[index] = [center[0], center[1], 0.0, 1.0];
        uniform.eye_inv[index] = inv;
        uniform.eye_extra[index] = [hw, hh, open, if visible { 1.0 } else { 0.0 }];
        uniform.eye_style[index] = [eye.corner, 0.0, 0.0, 0.0];
    }

    if let Some(notif) = frame.decor.notif {
        uniform.notif = [notif.x, -notif.y, notif.notch_r, 1.0];
    }

    uniform
}

fn build_overlay(
    frame: &BloubFrame,
    palette: BloubPalette,
    extent: f32,
) -> (Vec<OverlayVertex>, Vec<OverlayVertex>) {
    let mut back = Vec::new();
    let mut front = Vec::new();

    for arc in &frame.decor.arcs {
        let target = if arc.behind { &mut back } else { &mut front };
        push_arc(target, arc, extent);
    }

    for dot in &frame.decor.dots {
        let target = if dot.behind { &mut back } else { &mut front };
        push_dot(target, dot, palette, extent);
    }

    if let Some(notif) = frame.decor.notif {
        push_notification(&mut front, notif, extent);
    }

    (back, front)
}

fn push_arc(target: &mut Vec<OverlayVertex>, arc: &ArcRender, extent: f32) {
    for pair in arc.points.windows(2) {
        let p0 = pair[0];
        let p1 = pair[1];
        let dx = p1.x - p0.x;
        let dy = p1.y - p0.y;
        let len = (dx * dx + dy * dy).sqrt().max(1e-6);
        let nx = -dy / len * arc.width * 0.5;
        let ny = dx / len * arc.width * 0.5;
        let a = ndc(p0.x + nx, p0.y + ny, extent);
        let b = ndc(p0.x - nx, p0.y - ny, extent);
        let c = ndc(p1.x + nx, p1.y + ny, extent);
        let d = ndc(p1.x - nx, p1.y - ny, extent);
        let c0 = [
            p0.color[0],
            p0.color[1],
            p0.color[2],
            p0.color[3] * arc.opacity,
        ];
        let c1 = [
            p1.color[0],
            p1.color[1],
            p1.color[2],
            p1.color[3] * arc.opacity,
        ];
        target.extend_from_slice(&[
            OverlayVertex {
                position: a,
                color: c0,
            },
            OverlayVertex {
                position: b,
                color: c0,
            },
            OverlayVertex {
                position: c,
                color: c1,
            },
            OverlayVertex {
                position: b,
                color: c0,
            },
            OverlayVertex {
                position: d,
                color: c1,
            },
            OverlayVertex {
                position: c,
                color: c1,
            },
        ]);
    }
}

fn push_dot(target: &mut Vec<OverlayVertex>, dot: &Dot, palette: BloubPalette, extent: f32) {
    let color = dot.color.unwrap_or(palette.body);
    let alpha = dot.opacity * dot.depth * color[3];
    push_disc(target, dot.x, dot.y, dot.r, color, alpha, extent, 18);
}

fn push_disc(
    target: &mut Vec<OverlayVertex>,
    x: f32,
    y: f32,
    r: f32,
    color: [f32; 4],
    alpha: f32,
    extent: f32,
    segments: usize,
) {
    let center = ndc(x, y, extent);
    let c = [color[0], color[1], color[2], alpha];
    for i in 0..segments {
        let a0 = i as f32 / segments as f32 * TAU;
        let a1 = (i + 1) as f32 / segments as f32 * TAU;
        target.extend_from_slice(&[
            OverlayVertex {
                position: center,
                color: c,
            },
            OverlayVertex {
                position: ndc(x + a0.cos() * r, y + a0.sin() * r, extent),
                color: c,
            },
            OverlayVertex {
                position: ndc(x + a1.cos() * r, y + a1.sin() * r, extent),
                color: c,
            },
        ]);
    }
}

fn push_rect(
    target: &mut Vec<OverlayVertex>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: [f32; 4],
    alpha: f32,
    extent: f32,
) {
    let c = [color[0], color[1], color[2], alpha];
    let l = x - w * 0.5;
    let r = x + w * 0.5;
    let t = y - h * 0.5;
    let b = y + h * 0.5;
    let p0 = ndc(l, t, extent);
    let p1 = ndc(r, t, extent);
    let p2 = ndc(l, b, extent);
    let p3 = ndc(r, b, extent);
    target.extend_from_slice(&[
        OverlayVertex {
            position: p0,
            color: c,
        },
        OverlayVertex {
            position: p2,
            color: c,
        },
        OverlayVertex {
            position: p1,
            color: c,
        },
        OverlayVertex {
            position: p1,
            color: c,
        },
        OverlayVertex {
            position: p2,
            color: c,
        },
        OverlayVertex {
            position: p3,
            color: c,
        },
    ]);
}

fn push_notification(target: &mut Vec<OverlayVertex>, notif: NotifRender, extent: f32) {
    push_disc(
        target,
        notif.x,
        notif.y,
        notif.r,
        notif.color,
        1.0,
        extent,
        24,
    );
    let white = [1.0, 1.0, 1.0, 1.0];
    push_rect(
        target,
        notif.x,
        notif.y - notif.r * 0.10,
        notif.r * 0.17,
        notif.r * 0.62,
        white,
        0.95,
        extent,
    );
    push_disc(
        target,
        notif.x,
        notif.y + notif.r * 0.40,
        notif.r * 0.11,
        white,
        0.95,
        extent,
        12,
    );
}

fn ndc(x: f32, y: f32, extent: f32) -> [f32; 3] {
    [x / extent, -y / extent, 0.0]
}
