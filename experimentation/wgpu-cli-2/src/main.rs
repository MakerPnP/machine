use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use std::f32::consts::TAU;
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
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PushConstants {
    mvp: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    light_pos: [f32; 3],
    light_intensity: f32,
    light_color: [f32; 3],
    _padding: f32,
}

struct RenderState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    width: u32,
    height: u32,
}

impl RenderState {
    async fn new(width: u32, height: u32) -> Result<Self, Box<dyn std::error::Error>> {
        // Create instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
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
                    required_features: wgpu::Features::PUSH_CONSTANTS,
                    required_limits: wgpu::Limits {
                        max_push_constant_size: 256, // Ensure we have enough space for our push constants
                        ..Default::default()
                    },
                    experimental_features: Default::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: Default::default(),
                }
            )
            .await?;

        // Load shaders
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[wgpu::PushConstantRange {
                stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
                range: 0..std::mem::size_of::<PushConstants>() as u32,
            }],
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
                front_face: wgpu::FrontFace::Cw, // Clockwise for CAD models
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
        cube_push: PushConstants,
        pyramid_push: PushConstants,
        model_push: PushConstants,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
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
                    depth_slice: None,
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
            render_pass.set_push_constants(
                wgpu::ShaderStages::VERTEX_FRAGMENT,
                0,
                bytemuck::cast_slice(&[cube_push]),
            );
            render_pass.set_vertex_buffer(0, cube_vertex_buffer.slice(..));
            render_pass.set_index_buffer(cube_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..cube_indices.len() as u32, 0, 0..1);

            // Render pyramid
            render_pass.set_push_constants(
                wgpu::ShaderStages::VERTEX_FRAGMENT,
                0,
                bytemuck::cast_slice(&[pyramid_push]),
            );
            render_pass.set_vertex_buffer(0, pyramid_vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(pyramid_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..pyramid_indices.len() as u32, 0, 0..1);

            // Render model
            render_pass.set_push_constants(
                wgpu::ShaderStages::VERTEX_FRAGMENT,
                0,
                bytemuck::cast_slice(&[model_push]),
            );
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

        let _ = self.device.poll(PollType::Wait { submission_index: None, timeout: None });
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
        Vertex { pos: [1.5, -1.0, -1.5], color: [0.5, 1.0, 0.0] }, // 1: back-right
        Vertex { pos: [1.5, -1.0, 1.5], color: [0.0, 1.0, 0.5] }, // 2: front-right
        Vertex { pos: [-1.5, -1.0, 1.5], color: [0.5, 1.0, 0.5] }, // 3: front-left
        // Apex vertex (yellow)
        Vertex { pos: [0.0, 2.0, 0.0], color: [1.0, 1.0, 0.0] }, // 4: apex
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
        0, 4, 1, // back face
        1, 4, 2, // right face
        2, 4, 3, // front face
        3, 4, 0, // left face
    ];

    // Initialize wgpu (these currently must be multiples of 256)
    let width = 1024u32;
    let height = 1024u32;

    let rt = tokio::runtime::Runtime::new()?;
    let render_state = rt.block_on(RenderState::new(width, height))?;

    // Create transformation matrices
    let aspect = width as f32 / height as f32;
    let ortho_size = 10.0;
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

    for frame_index in 0..FRAME_COUNT {
        let t = frame_index as f32 / FRAME_COUNT as f32;

        // --- Light Animation ---
        let light_orbit_radius = 8.0;
        let light_height = 5.0;
        let light_angle = t * TAU;
        let light_pos = Vec3::new(
            light_orbit_radius * light_angle.cos(),
            light_height + 2.0 * (t * TAU * 2.0).sin(),
            light_orbit_radius * light_angle.sin(),
        );

        let light_intensity = 1.0 + 0.5 * (t * TAU * 3.0).sin();
        let light_hue = (t * 0.5) % 1.0;
        let light_color = hsv_to_rgb(light_hue, 0.8, 1.0);

        // --- Rotation ---
        let cube_rotation = t * TAU;
        let cube_model = Mat4::from_rotation_y(cube_rotation);

        let pyramid_rotation = -t * TAU;
        let pyramid_model = Mat4::from_translation(Vec3::new(3.0, 0.0, 0.0))
            * Mat4::from_rotation_y(pyramid_rotation);

        let model_rotation = t * TAU;
        let model_model = Mat4::from_rotation_x(model_rotation);

        // --- Zoom ---
        let base_distance = 6.0;
        let zoom_amplitude = 2.0;
        let zoom = base_distance - zoom_amplitude * (1.0 - (TAU * t).cos()) * 0.5;

        let view = Mat4::look_at_rh(Vec3::new(zoom, zoom * 0.75, zoom), Vec3::ZERO, Vec3::Y);

        let cube_mvp = projection * view * cube_model;
        let pyramid_mvp = projection * view * pyramid_model;
        let model_mvp = projection * view * model_model;

        // Create push constants
        let cube_push_constants = PushConstants {
            mvp: cube_mvp.to_cols_array_2d(),
            model: cube_model.to_cols_array_2d(),
            light_pos: light_pos.to_array(),
            light_intensity,
            light_color,
            _padding: 0.0,
        };

        let pyramid_push_constants = PushConstants {
            mvp: pyramid_mvp.to_cols_array_2d(),
            model: pyramid_model.to_cols_array_2d(),
            light_pos: light_pos.to_array(),
            light_intensity,
            light_color,
            _padding: 0.0,
        };

        let model_push_constants = PushConstants {
            mvp: model_mvp.to_cols_array_2d(),
            model: model_model.to_cols_array_2d(),
            light_pos: light_pos.to_array(),
            light_intensity,
            light_color,
            _padding: 0.0,
        };

        println!("Rendering frame {}", frame_index);

        let bytes = render_state.render_frame(
            &cube_vertices,
            &cube_indices,
            &pyramid_vertices,
            &pyramid_indices,
            &model_vertices,
            &model_indices,
            cube_push_constants,
            pyramid_push_constants,
            model_push_constants,
        )?;

        println!("Sending frame {}", frame_index);
        tx.send((frame_index, bytes))?;
    }

    drop(tx);
    let _ = handle.join().unwrap();

    Ok(())
}

fn load_step_model_unfinished(
    path: &str,
) -> Result<(Vec<Vertex>, Vec<u16>), Box<dyn std::error::Error>> {
    println!("Loading STEP file: {}", path);

    let step_data = std::fs::read_to_string(path)?;
    let exchange = truck_stepio::r#in::ruststep::parser::parse(&step_data)?;

    println!("Parsing complete, extracting shells...");

    let table: Table = Table::from_data_section(&exchange.data[0]);

    println!("Found {} shells", table.shell.len());

    if table.shell.is_empty() {
        return Err("No geometry found in STEP file".into());
    }

    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();
    let mut vertex_offset = 0u16;

    let tol = 0.01;

    let mut bounds = BoundingBox::new();
    for (shell_idx, (shell_id, shell)) in table.shell.iter().enumerate() {
        println!("Processing shell {}/{}", shell_idx + 1, table.shell.len());

        let Ok(shell) = table.to_compressed_shell(shell) else {
            println!("Failed to convert shell {} to polygon mesh", shell_id);
            continue;
        };

        let mesh: PolygonMesh = shell.robust_triangulation(tol).to_polygon();
        let mesh_bounds = mesh.bounding_box();
        bounds = bounds.add(mesh_bounds);

        println!(
            "  Vertices: {}, Faces: {}",
            mesh.positions().len(),
            mesh.faces().len()
        );

        let hue = (shell_idx as f32 * 0.618033988749895) % 1.0;
        let color = hsv_to_rgb(hue, 0.7, 0.9);

        for pos in mesh.positions() {
            all_vertices.push(Vertex {
                pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32],
                color,
            });
        }

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

    let (min, max) = (bounds.min(), bounds.max());
    let center = Vec3::new(
        (min[0] + max[0]) as f32 * 0.5,
        (min[1] + max[1]) as f32 * 0.5,
        (min[2] + max[2]) as f32 * 0.5,
    );

    let size = Vec3::new(
        (max[0] - min[0]) as f32,
        (max[1] - min[1]) as f32,
        (max[2] - min[2]) as f32,
    );
    let max_size = size.x.max(size.y).max(size.z);
    let scale = if max_size > 0.0 { 4.0 / max_size } else { 1.0 };

    println!("Model bounds: min={:?}, max={:?}", min, max);
    println!("Centering at {:?}, scaling by {}", center, scale);

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