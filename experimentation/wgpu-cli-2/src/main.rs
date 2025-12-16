use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use std::f32::consts::TAU;
use std::num::NonZeroU64;
use std::ops::Add;
use truck_meshalgo::prelude::{BoundingBox, MeshedShape, RobustMeshableShape};
use truck_polymesh::PolygonMesh;
use truck_stepio::r#in::Table;
use wgpu::{PollType, TexelCopyBufferLayout, TexelCopyTextureInfo};
use wgpu::util::DeviceExt;

const PROJECTION_STYLE: ProjectionStyle = ProjectionStyle::Normal;
const FRAME_COUNT: usize = 30;
const QUEUE_CAPACITY: usize = 30;

#[allow(dead_code)]
enum ProjectionStyle {
    Normal,
    Orthographic,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Pod, Zeroable)]
struct UniformData {
    mvp: glam::Mat4,
    model: glam::Mat4,
    light_pos: glam::Vec3,
    _padding1: u32,  // Explicit padding for light_pos
    light_color: glam::Vec3,
    _padding2: u32,  // Explicit padding for light_pos
    light_intensity: f32,
    _padding3: u32,
    _padding4: u32,
    _padding5: u32,
}

impl UniformData {
    pub fn new(
        mvp: glam::Mat4,
        model: glam::Mat4,
        light_pos: glam::Vec3,
        light_color: glam::Vec3,
        light_intensity: f32
    ) -> Self {
        Self {
            mvp,
            model,
            light_pos,
            _padding1: 0,
            light_color,
            _padding2: 0,
            light_intensity,
            _padding3: 0,
            _padding4: 0,
            _padding5: 0,
        }
    }
}

const UNIFORM_DATA_SIZE: usize = size_of::<UniformData>();
#[cfg(test)]
const UNIFORM_DATA_ALIGN: usize = align_of::<UniformData>();

#[cfg(test)]
mod uniform_data_tests {
    use super::*;
    #[test]
    fn uniform_data_size() {
        assert_eq!(size_of::<UniformData>(), UNIFORM_DATA_SIZE);
        assert_eq!(align_of::<UniformData>(), UNIFORM_DATA_ALIGN);
        assert_eq!(size_of::<UniformData>() % UNIFORM_DATA_ALIGN, 0, "Uniform data size must be a multiple of {}", UNIFORM_DATA_ALIGN);
        assert_eq!(align_of::<UniformData>(), 16);
    }

    #[test]
    fn uniform_data_alignment() {
        // Check offsets
        assert_eq!(memoffset::offset_of!(UniformData, mvp), 0);
        assert_eq!(memoffset::offset_of!(UniformData, model), 64);
        assert_eq!(memoffset::offset_of!(UniformData, light_pos), 128);
        assert_eq!(memoffset::offset_of!(UniformData, light_color), 144);
        assert_eq!(memoffset::offset_of!(UniformData, light_intensity), 160);
    }
}

struct RenderState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    width: u32,
    height: u32,
}

impl RenderState {
    async fn new(width: u32, height: u32) -> Result<Self, Box<dyn std::error::Error>> {
        // Create instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::from_env().unwrap_or_default(),
            ..Default::default()
        });

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await?;

        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits {
                        // raspberry pi 5 gpu only allows 4096 max.
                        max_texture_dimension_2d: 4096,
                        // raspberry pi 5 gpu only allows 4096 max.
                        max_texture_dimension_1d: 4096,
                        ..Default::default()
                    },
//                    experimental_features: Default::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: Default::default(),
                }
            )
            .await?;

        println!("Adapter: {:?}", adapter.get_info());

        // Load shaders
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Uniform Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(NonZeroU64::new(UNIFORM_DATA_SIZE as u64).unwrap()), // Specify size
                    },
                    count: None,
                },
            ],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
            width,
            height,
        })
    }

    fn render_frame(
        &self,
        cube_vertices: &[Vertex],
        cube_indices: &[u16],
        pyramid_vertices: &[Vertex],
        pyramid_indices: &[u16],
        model_vertices: &[Vertex],
        model_indices: &[u16],
        cube_push: UniformData,
        pyramid_push: UniformData,
        model_push: UniformData,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let cube_uniform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Uniform Buffer"),
            contents: bytemuck::cast_slice(&[cube_push]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        // Create vertex buffers
        let cube_vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cube Vertex Buffer"),
                contents: bytemuck::cast_slice(cube_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let cube_index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cube Index Buffer"),
                contents: bytemuck::cast_slice(cube_indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let pyramid_uniform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pyramid Uniform Buffer"),
            contents: bytemuck::cast_slice(&[pyramid_push]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let pyramid_vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pyramid Vertex Buffer"),
                contents: bytemuck::cast_slice(pyramid_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let pyramid_index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Pyramid Index Buffer"),
                contents: bytemuck::cast_slice(pyramid_indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        let model_uniform_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Model Uniform Buffer"),
            contents: bytemuck::cast_slice(&[model_push]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let model_vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Model Vertex Buffer"),
                contents: bytemuck::cast_slice(model_vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let model_index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Model Index Buffer"),
                contents: bytemuck::cast_slice(model_indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        // Create bind groups for each object
        let cube_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cube Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cube_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let pyramid_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pyramid Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: pyramid_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let model_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Model Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: model_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Create render target texture
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Render Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create depth texture
        let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
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

        // Create output buffer for reading back
        let output_buffer_size = (self.width * self.height * 4) as u64;
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);

            // Render cube
            render_pass.set_bind_group(0, &cube_bind_group, &[]);
            render_pass.set_vertex_buffer(0, cube_vertex_buffer.slice(..));
            render_pass.set_index_buffer(cube_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..cube_indices.len() as u32, 0, 0..1);

            // Render pyramid
            render_pass.set_bind_group(0, &pyramid_bind_group, &[]);
            render_pass.set_vertex_buffer(0, pyramid_vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(pyramid_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..pyramid_indices.len() as u32, 0, 0..1);

            // Render model
            render_pass.set_bind_group(0, &model_bind_group, &[]);
            render_pass.set_vertex_buffer(0, model_vertex_buffer.slice(..));
            render_pass.set_index_buffer(model_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..model_indices.len() as u32, 0, 0..1);
        }

        // Copy texture to buffer
        let u32_size = size_of::<u32>() as u32;

        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(u32_size * self.width),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));

        // Read back the data
        let buffer_slice = output_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

//        let _ = self.device.poll(PollType::Wait { submission_index: None, timeout: None });
        let _ = self.device.poll(PollType::Wait);
        receiver.recv()??;

        let data = buffer_slice.get_mapped_range();
        let result = data.to_vec();
        drop(data);
        output_buffer.unmap();

        Ok(result)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load STEP model
    let step_path = "assets/User Library-LQFP64_Grid_0_5.STEP";
    let (model_vertices, model_indices) = load_step_model_unfinished(step_path)?;

    //
    // For the models here, Y+ = UP
    //

    // Define cube vertices with colors
    let cube_vertices = [
        // Front face (red)
        Vertex { pos: [-1.0, -1.0, 1.0], color: [1.0, 0.0, 0.0] },
        Vertex { pos: [1.0, -1.0, 1.0], color: [1.0, 0.5, 0.0] },
        Vertex { pos: [1.0, 1.0, 1.0], color: [1.0, 1.0, 0.0] },
        Vertex { pos: [-1.0, 1.0, 1.0], color: [1.0, 0.5, 0.5] },
        // Back face (blue)
        Vertex { pos: [-1.0, -1.0, -1.0], color: [0.0, 0.0, 1.0] },
        Vertex { pos: [1.0, -1.0, -1.0], color: [0.0, 0.5, 1.0] },
        Vertex { pos: [1.0, 1.0, -1.0], color: [0.5, 0.5, 1.0] },
        Vertex { pos: [-1.0, 1.0, -1.0], color: [0.5, 0.0, 1.0] },
    ];

    // Define pyramid vertices with colors
    let pyramid_vertices = [
        // Base vertices (green)
        Vertex { pos: [-1.5, -1.0, -1.5], color: [0.0, 1.0, 0.0] }, // 0: back-left
        Vertex { pos: [1.5, -1.0, -1.5], color: [0.5, 1.0, 0.0] },  // 1: back-right
        Vertex { pos: [1.5, -1.0, 1.5], color: [0.0, 1.0, 0.5] },   // 2: front-right
        Vertex { pos: [-1.5, -1.0, 1.5], color: [0.5, 1.0, 0.5] },  // 3: front-left
        // Apex vertex (yellow)
        Vertex { pos: [0.0, 2.0, 0.0], color: [1.0, 1.0, 0.0] },    // 4: apex
    ];

    // Define cube indices
    let cube_indices: [u16; 36] = [
        0, 1, 2, 2, 3, 0, // Front
        1, 5, 6, 6, 2, 1, // Right
        5, 4, 7, 7, 6, 5, // Back
        4, 0, 3, 3, 7, 4, // Left
        3, 2, 6, 6, 7, 3, // Top
        4, 5, 1, 1, 0, 4, // Bottom
    ];

    // Define pyramid indices
    let pyramid_indices: [u16; 18] = [
        // Base (2 triangles)
        0, 1, 2,
        0, 2, 3,
        // Sides (4 triangles)
        0, 4, 1,  // back face
        1, 4, 2,  // right face
        2, 4, 3,  // front face
        3, 4, 0,  // left face
    ];

    // Initialize wgpu (these currently must be multiples of 256)
    let width = 1024u32;
    let height = 1024u32;

    let rt = tokio::runtime::Runtime::new()?;
    let render_state = rt.block_on(RenderState::new(width, height))?;

    // Create transformation matrices
    let aspect = width as f32 / height as f32;
    let ortho_size = 10.0;  // How many world units fit in view
    let projection = match PROJECTION_STYLE {
        ProjectionStyle::Normal => Mat4::perspective_rh(45.0_f32.to_radians(), aspect, 0.1, 100.0),
        ProjectionStyle::Orthographic => Mat4::orthographic_rh(
            -ortho_size * aspect,
            ortho_size * aspect,
            -ortho_size,
            ortho_size,
            0.1,
            100.0,
        ),
    };

    let (tx, rx) =
        std::sync::mpsc::sync_channel::<(usize, Vec<u8>)>(QUEUE_CAPACITY);

    let handle = std::thread::spawn(move || {
        while let Ok((frame_index, bytes_vec)) = rx.recv() {
            println!("Received frame {}", frame_index);

            let start_at = std::time::Instant::now();

            // Save to PNG
            image::save_buffer(
                format!("assets/cube_{:03}.png", frame_index),
                &bytes_vec,
                width,
                height,
                image::ColorType::Rgba8,
            )
                .map_err(|e| {
                    println!("Failed to save frame {}, e: {}", frame_index, e);
                })?;

            println!(
                "✓ Saved frame {}, elapsed: {}us",
                frame_index,
                start_at.elapsed().as_micros()
            );
        }

        Ok::<(), ()>(())
    });

    let mut frame_index = 0;
    loop {
        let t = frame_index as f32 / FRAME_COUNT as f32;

        // --- Light Animation ---
        // Orbit the light around the scene
        let light_orbit_radius = 8.0;
        let light_height = 5.0;
        let light_angle = t * TAU;
        let light_pos = Vec3::new(
            light_orbit_radius * light_angle.cos(),
            light_height + 2.0 * (t * TAU * 2.0).sin(), // Also animate height
            light_orbit_radius * light_angle.sin(),
        );

        // Animate light intensity (pulse between 0.5 and 2.0)
        let light_intensity = 1.0 + 0.5 * (t * TAU * 3.0).sin();

        // Animate light color (shift through colors)
        let light_hue = (t * 0.5) % 1.0;
        let light_color = hsv_to_rgb(light_hue, 0.8, 1.0);

        // --- Rotation ---
        let cube_rotation = t * TAU;
        let cube_model = Mat4::from_rotation_y(cube_rotation);

        // Pyramid rotates in opposite direction
        let pyramid_rotation = -t * TAU;
        // Position pyramid to the side and rotate it
        let pyramid_model = Mat4::from_translation(Vec3::new(3.0, 0.0, 0.0))
            * Mat4::from_rotation_y(pyramid_rotation);

        // --- Rotation ---
        let model_rotation = t * TAU;
        let model_model = Mat4::from_rotation_x(model_rotation);

        // --- Zoom (smooth in/out) ---
        let base_distance = 6.0;
        let zoom_amplitude = 2.0;
        let zoom = base_distance
            - zoom_amplitude * (1.0 - (TAU * t).cos()) * 0.5;

        let view = Mat4::look_at_rh(
            Vec3::new(zoom, zoom * 0.75, zoom),
            Vec3::ZERO,
            Vec3::Y,
        );

        let cube_mvp = projection * view * cube_model;
        let pyramid_mvp = projection * view * pyramid_model;
        let model_mvp = projection * view * model_model;

        // Create uniform data with lighting info

        let cube_uniform_data = UniformData::new(
            cube_mvp,
            cube_model,
            glam::Vec3::new(light_pos.x, light_pos.y, light_pos.z),
            glam::Vec3::new(light_color[0], light_color[1], light_color[2]),
            light_intensity,
        );

        let pyramid_uniform_data = UniformData::new(
            pyramid_mvp,
            pyramid_model,
            glam::Vec3::new(light_pos.x, light_pos.y, light_pos.z),
            glam::Vec3::new(light_color[0], light_color[1], light_color[2]),
            light_intensity,
        );

        let model_uniform_data = UniformData::new(
            model_mvp,
            model_model,
            glam::Vec3::new(light_pos.x, light_pos.y, light_pos.z),
            glam::Vec3::new(light_color[0], light_color[1], light_color[2]),
            light_intensity,
        );

        println!("Rendering frame {}", frame_index);

        let bytes = render_state.render_frame(
            &cube_vertices,
            &cube_indices,
            &pyramid_vertices,
            &pyramid_indices,
            &model_vertices,
            &model_indices,
            cube_uniform_data,
            pyramid_uniform_data,
            model_uniform_data,
        )?;

        println!("Sending frame {}", frame_index);
        tx.send((frame_index, bytes))?;

        frame_index += 1;
        if frame_index >= FRAME_COUNT {
            break;
        }
    }

    // explictly drop the sender to make sure the receiver is dropped before the thread exits
    drop(tx);

    let _ = handle.join().unwrap();

    Ok(())
}

// FIXME this is wrong, we need to return something so we can use GPU instancing of the shells
fn load_step_model_unfinished(
    path: &str,
) -> Result<(Vec<Vertex>, Vec<u16>), Box<dyn std::error::Error>> {
    println!("Loading STEP file: {}", path);

    // Read STEP file
    let step_data = std::fs::read_to_string(path)?;
    let exchange = truck_stepio::r#in::ruststep::parser::parse(&step_data)?;

    println!("Parsing complete, extracting shells...");

    // Extract shells from the STEP model
    let table: Table = Table::from_data_section(&exchange.data[0]);

    println!("Found {} shells", table.shell.len());

    if table.shell.is_empty() {
        return Err("No geometry found in STEP file".into());
    }

    // Collect all vertices and indices
    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();
    let mut vertex_offset = 0u16;

    // Mesh parameters - adjust for quality vs performance
    let tol = 0.01; // Tolerance for meshing

    let mut bounds = BoundingBox::new();
    for (shell_idx, (shell_id, shell)) in table.shell.iter().enumerate() {
        println!("Processing shell {}/{}, id: {}", shell_idx + 1, table.shell.len(), shell_id);

        let Ok(shell) = table.to_compressed_shell(shell) else {
            println!("Failed to convert shell {} to polygon mesh", shell_id);
            continue
        };

        let mesh: PolygonMesh = shell.robust_triangulation(tol).to_polygon();
        let mesh_bounds = mesh.bounding_box();
        bounds = bounds.add(mesh_bounds);

        println!(
            "  Vertices: {}, Faces: {}",
            mesh.positions().len(),
            mesh.faces().len()
        );

        // Calculate a color based on shell id
        let hue = (*shell_id as f32 * 0.618033988749895) % 1.0;  // Golden ratio for distribution
        let color = hsv_to_rgb(hue, 0.7, 0.9);

        // Add vertices
        for pos in mesh.positions() {
            all_vertices.push(Vertex {
                pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32],
                color,
            });
        }

        // Add indices (triangulated faces)
        for face_indices in mesh.faces().triangle_iter() {
            for &idx in &face_indices {
                all_indices.push(vertex_offset + idx.pos as u16);
            }
        }

        vertex_offset += mesh.positions().len() as u16;
    }

    println!(
        "Total vertices: {}, Total indices: {}",
        all_vertices.len(),
        all_indices.len()
    );

    // Calculate bounding box for centering
    let (min, max) = (bounds.min(), bounds.max());
    let center = Vec3::new(
        (min[0] + max[0]) as f32 * 0.5,
        (min[1] + max[1]) as f32 * 0.5,
        (min[2] + max[2]) as f32 * 0.5,
    );

    // Calculate scale to fit in view
    let size = Vec3::new(
        (max[0] - min[0]) as f32,
        (max[1] - min[1]) as f32,
        (max[2] - min[2]) as f32,
    );
    let max_size = size.x.max(size.y).max(size.z);
    let scale = if max_size > 0.0 { 4.0 / max_size } else { 1.0 };

    println!("Model bounds: min={:?}, max={:?}", min, max);
    println!("Centering at {:?}, scaling by {}", center, scale);

    // Center and scale vertices
    for vertex in &mut all_vertices {
        vertex.pos[0] = (vertex.pos[0] - center.x) * scale;
        vertex.pos[1] = (vertex.pos[1] - center.y) * scale;
        vertex.pos[2] = (vertex.pos[2] - center.z) * scale;
    }

    Ok((all_vertices, all_indices))
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 1.0 / 6.0 {
        (c, x, 0.0)
    } else if h < 2.0 / 6.0 {
        (x, c, 0.0)
    } else if h < 3.0 / 6.0 {
        (0.0, c, x)
    } else if h < 4.0 / 6.0 {
        (0.0, x, c)
    } else if h < 5.0 / 6.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    [r + m, g + m, b + m]
}
