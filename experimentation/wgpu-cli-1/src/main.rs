use std::collections::HashMap;
use std::path::Path;

use wgpu::util::DeviceExt;
use glam::{Mat4, Vec3};
use truck_meshalgo::tessellation::{MeshedShape, RobustMeshableShape};
use truck_polymesh::PolygonMesh;
use truck_stepio::r#in::ruststep::ast::{EntityInstance, Exchange};
use truck_stepio::r#in::{ruststep, Table};

use wgpu::{PollType, TexelCopyBufferLayout, TexelCopyTextureInfo};
use log::trace;

// Vertex structure for our 3D mesh
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
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

// Uniform data for camera and lighting
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    light_pos: [f32; 3],
    _padding: f32,
    light_color: [f32; 3],
    _padding2: f32,
    object_color: [f32; 3],
    _padding3: f32,
}

struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    width: u32,
    height: u32,
}

impl Renderer {
    async fn new(width: u32, height: u32) -> Self {
        // Create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    experimental_features: Default::default(),
                    memory_hints: Default::default(),
                    trace: Default::default(),
                }
            )
            .await
            .expect("Failed to create device");

        // Shader code
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("bind_group_layout"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            device,
            queue,
            render_pipeline,
            width,
            height,
        }
    }

    fn render_to_image(
        &self,
        vertices: &[Vertex],
        indices: &[u32],
        camera_pos: Vec3,
        target_pos: Vec3,
        light_pos: Vec3,
        object_color: [f32; 3],
    ) -> Vec<u8> {
        // Create vertex and index buffers
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        // Set up camera matrices
        let view = Mat4::look_at_rh(camera_pos, target_pos, Vec3::Y);
        let proj = Mat4::perspective_rh(
            45.0_f32.to_radians(),
            self.width as f32 / self.height as f32,
            0.1,
            100.0,
        );
        let view_proj = proj * view;

        let uniforms = Uniforms {
            view_proj: view_proj.to_cols_array_2d(),
            model: Mat4::IDENTITY.to_cols_array_2d(),
            light_pos: light_pos.to_array(),
            _padding: 0.0,
            light_color: [1.0, 1.0, 1.0],
            _padding2: 0.0,
            object_color,
            _padding3: 0.0,
        };

        let uniform_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Uniform Buffer"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group_layout = self.render_pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("bind_group"),
        });

        // Create render target texture
        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        };
        let texture = self.device.create_texture(&texture_desc);
        let texture_view = texture.create_view(&Default::default());

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
        let depth_view = depth_texture.create_view(&Default::default());

        // Create output buffer
        let u32_size = size_of::<u32>() as u32;
        let output_buffer_size = (u32_size * self.width * self.height) as wgpu::BufferAddress;
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        });

        // Render
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

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

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }

        // Copy texture to buffer
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
            texture_desc.size,
        );

        self.queue.submit(Some(encoder.finish()));

        // Read buffer
        let buffer_slice = output_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

        let _ = self.device.poll(PollType::Wait { submission_index: None, timeout: None });

        if let Ok(Ok(())) = receiver.recv() {
            let data = buffer_slice.get_mapped_range();
            let result = data.to_vec();
            drop(data);
            output_buffer.unmap();
            result
        } else {
            panic!("Failed to read buffer");
        }
    }
}

/// WARNING: Some step files currently result in some strange artifacts, this code is probably WRONG
///          do not use as a reference.
fn load_step(path: &Path) -> (Vec<Vertex>, Vec<u32>) {

    println!("Parsing STEP file (this may take a moment)...");

    // Read STEP file
    let step_string = std::fs::read_to_string(path)
        .expect("Failed to read STEP file");

    // Parse STEP into shapes
    let exchange: Exchange = ruststep::parser::parse(&step_string)
        .expect("Failed to parse STEP file");


    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();

    // Tessellation tolerance - smaller = more detailed mesh
    let tolerance = 0.001;

    // well we can find some colors, but we don't have any relationship, so, ...
    let _color_map = exchange.data[0].entities.iter().filter_map(|entity| {
        match entity {
            s @ EntityInstance::Simple { id, record, .. } if record.name == "COLOUR_RGB" => {
                trace!("id: {}, simple: {:?}", id, s);
                Some((id, &record.parameter))
            }
            _ => None
        }
    }).collect::<HashMap<_, _>>();

    let table: Table = Table::from_data_section(&exchange.data[0]);

    let polygons: Vec<(_, PolygonMesh)> = table
        .shell
        .iter()
        .map(|(idx, shell)| {
            let shell = table.to_compressed_shell(shell).unwrap();
            let pre = shell.robust_triangulation(tolerance).to_polygon();
            let bdd = pre.bounding_box();
            let compressed_shell = shell.robust_triangulation(bdd.diameter() * tolerance);
            (idx, compressed_shell.to_polygon())
        })
        .collect::<Vec<_>>();

    let len = table.shell.len();
    println!("Processing {} shapes", len);

    for (shape_idx, (idx, polygon_mesh)) in polygons.iter().enumerate() {
        println!("Processing shape {}/{}, idx: {}", shape_idx + 1, len, idx);

        let positions = polygon_mesh.positions();
        let normals = polygon_mesh.normals();
        let indices = polygon_mesh.tri_faces();

        // Make sure vertex count matches
        assert_eq!(positions.len(), normals.len());

        // Base offset for this batch of vertices
        let base = all_vertices.len() as u32;

        // Push vertices
        for (p, n) in positions.iter().zip(normals.iter()) {
            all_vertices.push(Vertex {
                position: [p.x as f32, p.y as f32, p.z as f32],
                normal:   [n.x as f32, n.y as f32, n.z as f32],
            });
        }

        // Push triangle indices, offset by base
        for tri in indices {
            all_indices.push(base + tri[0].pos as u32);
            all_indices.push(base + tri[1].pos as u32);
            all_indices.push(base + tri[2].pos as u32);
        }
    }

    (all_vertices, all_indices)
}

fn load_model(path: &Path) -> (Vec<Vertex>, Vec<u32>) {
    let extension = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match extension.as_deref() {
        Some("step") | Some("stp") => {
            println!("Loading STEP file...");
            load_step(path)
        }
        _ => {
            panic!("Unsupported file format. Supported formats: .step, .stp");
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <input.step|input.stp> <output.png>", args[0]);
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);

    println!("Loading 3D model: {:?}", input_path);
    let (vertices, indices) = load_model(input_path);
    println!("Loaded {} vertices, {} triangles", vertices.len(), indices.len() / 3);

    println!("Initializing renderer...");
    let renderer = pollster::block_on(Renderer::new(1024, 1024));

    println!("Rendering...");
    let image_data = renderer.render_to_image(
        &vertices,
        &indices,
        Vec3::new(3.0, 5.0, 25.0),  // Camera position
        Vec3::new(0.0, 0.0, 0.0),  // Look at target
        Vec3::new(5.0, 5.0, 5.0),  // Light position
        [0.6, 0.8, 0.9],           // Object color (light blue)
    );

    println!("Saving image to: {:?}", output_path);
    image::save_buffer(
        output_path,
        &image_data,
        1024,
        1024,
        image::ColorType::Rgba8,
    )
        .expect("Failed to save image");

    println!("Done!");
}