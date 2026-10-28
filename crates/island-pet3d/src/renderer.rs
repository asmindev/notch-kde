use std::borrow::Cow;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use slint::wgpu_30::wgpu;
use wgpu::util::DeviceExt;

use crate::loader::{
    compute_worlds, load_momo, Material, MeshAsset, NodeKind, Vertex,
};
use crate::Pet3dState;

const SUPERSAMPLE: u32 = 2;
const TEXTURE_SIZE: u32 = 128;
const MAX_NODES: usize = 24;
const MAX_MATERIALS: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct SceneUniform {
    view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
    light_dirs: [[f32; 4]; 3],
    light_colors: [[f32; 4]; 3],
    ambient: [f32; 4],
    node_matrices: [[[f32; 4]; 4]; MAX_NODES],
    node_normal_matrices: [[[f32; 4]; 4]; MAX_NODES],
    materials: [Material; MAX_MATERIALS],
}

pub struct Pet3dRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    asset: MeshAsset,
    view_proj: Mat4,
    camera_pos: Vec3,

    scene_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    scene_bind_group: wgpu::BindGroup,
    composite_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,

    _supersample_texture: wgpu::Texture,
    supersample_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    final_texture: wgpu::Texture,
    final_view: wgpu::TextureView,
}

impl Pet3dRenderer {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> crate::loader::Result<Self> {
        let asset = load_momo()?;
        if asset.nodes.len() > MAX_NODES {
            return Err(format!(
                "momo.glb has {} nodes, renderer supports {MAX_NODES}",
                asset.nodes.len()
            )
            .into());
        }
        if asset.materials.len() > MAX_MATERIALS {
            return Err(format!(
                "momo.glb has {} materials, renderer supports {MAX_MATERIALS}",
                asset.materials.len()
            )
            .into());
        }

        let supersample_size = TEXTURE_SIZE * SUPERSAMPLE;
        let supersample_texture = create_color_texture(
            &device,
            "momo-supersample",
            supersample_size,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let supersample_view =
            supersample_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let final_texture = create_color_texture(
            &device,
            "momo-final",
            TEXTURE_SIZE,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let final_view = final_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("momo-depth"),
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

        let scene_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("momo-scene-bind-group-layout"),
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
                label: Some("momo-composite-bind-group-layout"),
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

        let scene_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("momo-scene-pipeline-layout"),
                bind_group_layouts: &[Some(&scene_bind_group_layout)],
                immediate_size: 0,
            });
        let composite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("momo-composite-pipeline-layout"),
                bind_group_layouts: &[Some(&composite_bind_group_layout)],
                immediate_size: 0,
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("momo-shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let scene_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("momo-scene-pipeline"),
            layout: Some(&scene_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_scene"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(Vertex::LAYOUT)],
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
                entry_point: Some("fs_scene"),
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

        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("momo-composite-pipeline"),
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

        let center = asset.center;
        let camera_height = asset.size.y.max(asset.size.x) * 1.10;
        let camera_pos = Vec3::new(center.x, center.y, center.z + 5.0);
        let view = Mat4::look_at_rh(camera_pos, center, Vec3::Y);
        let projection = Mat4::orthographic_rh(
            -camera_height * 0.5,
            camera_height * 0.5,
            -camera_height * 0.5,
            camera_height * 0.5,
            0.1,
            10.0,
        );
        let view_proj = projection * view;

        let mut uniform = SceneUniform::zeroed();
        uniform.view_proj = view_proj.to_cols_array_2d();
        uniform.camera_pos = camera_pos.extend(1.0).to_array();
        uniform.light_dirs = [
            Vec3::new(-0.45, 0.80, 0.65).normalize().extend(0.0).to_array(),
            Vec3::new(0.75, 0.20, 0.50).normalize().extend(0.0).to_array(),
            Vec3::new(0.20, 0.55, -0.80).normalize().extend(0.0).to_array(),
        ];
        uniform.light_colors = [
            [1.35, 1.28, 1.18, 0.0],
            [0.42, 0.52, 0.85, 0.0],
            [0.85, 0.72, 0.55, 0.0],
        ];
        uniform.ambient = [0.16, 0.17, 0.22, 0.0];

        for (index, node) in asset.nodes.iter().enumerate() {
            uniform.node_matrices[index] = node.rest_world.to_cols_array_2d();
            uniform.node_normal_matrices[index] =
                node.rest_world.inverse().transpose().to_cols_array_2d();
        }
        for (index, material) in asset.materials.iter().enumerate() {
            uniform.materials[index] = *material;
        }

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("momo-scene-uniform"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("momo-scene-bind-group"),
            layout: &scene_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("momo-composite-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("momo-composite-bind-group"),
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("momo-vertices"),
            contents: bytemuck::cast_slice(&asset.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("momo-indices"),
            contents: bytemuck::cast_slice(&asset.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let index_count = asset.indices.len() as u32;

        Ok(Self {
            device,
            queue,
            asset,
            view_proj,
            camera_pos,
            scene_pipeline,
            composite_pipeline,
            scene_bind_group,
            composite_bind_group,
            uniform_buffer,
            vertex_buffer,
            index_buffer,
            index_count,
            _supersample_texture: supersample_texture,
            supersample_view,
            depth_view,
            final_texture,
            final_view,
        })
    }

    pub fn image(&self) -> std::result::Result<slint::Image, slint::wgpu_30::TextureImportError> {
        slint::Image::try_from(self.final_texture.clone())
    }

    pub fn update(&mut self, state: Pet3dState, time: f32) {
        let locals = self.animated_locals(&state, time);
        let worlds = compute_worlds(&self.asset.nodes, &locals);

        let mut uniform = SceneUniform::zeroed();
        uniform.view_proj = self.view_proj.to_cols_array_2d();
        uniform.camera_pos = self.camera_pos.extend(1.0).to_array();
        uniform.light_dirs = [
            Vec3::new(-0.45, 0.80, 0.65).normalize().extend(0.0).to_array(),
            Vec3::new(0.75, 0.20, 0.50).normalize().extend(0.0).to_array(),
            Vec3::new(0.20, 0.55, -0.80).normalize().extend(0.0).to_array(),
        ];
        uniform.light_colors = [
            [1.35, 1.28, 1.18, 0.0],
            [0.42, 0.52, 0.85, 0.0],
            [0.85, 0.72, 0.55, 0.0],
        ];
        uniform.ambient = [0.16, 0.17, 0.22, 0.0];

        for (index, world) in worlds.iter().enumerate() {
            uniform.node_matrices[index] = world.to_cols_array_2d();
            uniform.node_normal_matrices[index] =
                world.inverse().transpose().to_cols_array_2d();
        }
        for (index, material) in self.asset.materials.iter().enumerate() {
            uniform.materials[index] = *material;
        }

        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniform));
    }

    pub fn render(&self) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("momo-encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("momo-scene-pass"),
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

            pass.set_pipeline(&self.scene_pipeline);
            pass.set_bind_group(0, &self.scene_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("momo-composite-pass"),
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

    fn animated_locals(&self, state: &Pet3dState, time: f32) -> Vec<Mat4> {
        let mut locals: Vec<Mat4> = self.asset.nodes.iter().map(|node| node.local).collect();
        let bass = state.bass.clamp(0.0, 1.0);
        let happy = state.happy_age >= 0.0 && state.happy_age < 1.5;
        let welcome = state.welcome_age >= 0.0 && state.welcome_age < 3.5;

        if welcome {
            let age = state.welcome_age;
            if let Some(root) = self.asset.root {
                locals[root] = locals[root] * Mat4::from_rotation_y((age * 2.8).sin() * 0.10);
            }
            self.wave_arms(&mut locals, (age * 5.0).sin() * 0.55);
            self.squash_eyes(&mut locals, 0.88);
            self.set_mouth(&mut locals, 1.12);
            self.set_cheeks(&mut locals, 1.12);
            self.animate_spirit(&mut locals, age, 0.04);
        } else if happy {
            let age = state.happy_age;
            self.wave_arms(&mut locals, (age * 14.0).sin() * 0.65);
            self.squash_eyes(&mut locals, 0.80);
            self.set_mouth(&mut locals, 1.15);
            self.set_cheeks(&mut locals, 1.18);
            self.animate_spirit(&mut locals, age, 0.03);
        } else if state.is_singing {
            if let Some(root) = self.asset.root {
                locals[root] = locals[root] * Mat4::from_rotation_z((time * 2.4).sin() * 0.018);
            }
            self.set_mouth(&mut locals, 0.55 + bass * 1.15);
            self.animate_spirit(&mut locals, time, 0.035);
        }

        if !happy && !welcome {
            let blink = blink_scale(state.blink_age);
            if blink < 0.999 {
                self.squash_eyes(&mut locals, blink);
            }
        }

        self.apply_gaze(&mut locals, state.gaze_x, state.gaze_y);
        if let Some(root) = self.asset.root {
            locals[root] = locals[root] * Mat4::from_rotation_y(state.gaze_x * 0.16);
        }

        locals
    }

    fn wave_arms(&self, locals: &mut [Mat4], angle: f32) {
        if let Some(arm) = self.asset.arm_l {
            locals[arm] = locals[arm] * Mat4::from_rotation_z(angle);
        }
        if let Some(arm) = self.asset.arm_r {
            locals[arm] = locals[arm] * Mat4::from_rotation_z(-angle);
        }
    }

    fn squash_eyes(&self, locals: &mut [Mat4], factor: f32) {
        for group in &self.asset.eye_groups {
            let pivot = group.pivot;
            let transform = Mat4::from_translation(pivot)
                * Mat4::from_scale(Vec3::new(1.0, factor, 1.0))
                * Mat4::from_translation(-pivot);
            for &node in &group.nodes {
                locals[node] = transform * locals[node];
            }
        }
    }

    fn apply_gaze(&self, locals: &mut [Mat4], gaze_x: f32, gaze_y: f32) {
        let offset = Vec3::new(gaze_x * 0.060, -gaze_y * 0.040, 0.0);
        if offset.length_squared() < 1e-6 {
            return;
        }

        let transform = Mat4::from_translation(offset);
        for group in &self.asset.eye_groups {
            for &node in &group.gaze_nodes {
                locals[node] = transform * locals[node];
            }
        }
    }

    fn set_mouth(&self, locals: &mut [Mat4], factor: f32) {
        if let Some(mouth) = self.asset.mouth {
            locals[mouth] = locals[mouth] * Mat4::from_scale(Vec3::new(1.0, factor, 1.0));
        }
    }

    fn set_cheeks(&self, locals: &mut [Mat4], factor: f32) {
        for &cheek in &self.asset.cheeks {
            locals[cheek] = locals[cheek] * Mat4::from_scale(Vec3::splat(factor));
        }
    }

    fn animate_spirit(&self, locals: &mut [Mat4], time: f32, amplitude: f32) {
        if let Some(spirit) = self.asset.spirit {
            let bob = (time * 4.0).sin() * amplitude;
            let angle = (time * 2.2).sin() * 0.12;
            locals[spirit] = Mat4::from_translation(Vec3::new(0.0, bob, 0.0))
                * locals[spirit]
                * Mat4::from_rotation_y(angle);
        }
    }

    #[allow(dead_code)]
    fn node_kind(&self, index: usize) -> NodeKind {
        self.asset.nodes[index].kind
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
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn blink_scale(age: f32) -> f32 {
    const DURATION: f32 = 0.18;
    if !(0.0..DURATION).contains(&age) {
        return 1.0;
    }

    let t = age / DURATION;
    let closed = if t < 0.35 {
        t / 0.35
    } else if t < 0.65 {
        1.0
    } else {
        (1.0 - t) / 0.35
    };
    1.0 - closed.clamp(0.0, 1.0) * 0.90
}
