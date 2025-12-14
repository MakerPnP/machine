use bytemuck::{cast_slice, Pod, Zeroable};
use glam::{Mat4, Vec3};
use screen_13::prelude::*;
use std::sync::Arc;
use screen_13::prelude::vk::DeviceSize;

use std::f32::consts::TAU;
use std::ops::Add;
use truck_meshalgo::prelude::{BoundingBox, MeshedShape, RobustMeshableShape};
use truck_polymesh::PolygonMesh;
use truck_stepio::r#in::Table;

const FRAME_COUNT: usize = 30;
const QUEUE_CAPACITY: usize = 30;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 3],
    color: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PushConstants {
    mvp: [[f32; 4]; 4],
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load STEP model
    let step_path = "assets/User Library-LQFP64_Grid_0_5.STEP";
    let (model_vertices, model_indices) = load_step_model_unfinished(step_path)?;

    // Create headless device for offscreen rendering
    let device = Arc::new(Device::create_headless(DeviceInfo::default())?);

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

    // Create buffers for cube
    let cube_vertex_buf = Arc::new(Buffer::create_from_slice(
        &device,
        vk::BufferUsageFlags::VERTEX_BUFFER,
        cast_slice(&cube_vertices),
    )?);

    let cube_index_buf = Arc::new(Buffer::create_from_slice(
        &device,
        vk::BufferUsageFlags::INDEX_BUFFER,
        cast_slice(&cube_indices),
    )?);

    // Create buffers for pyramid
    let pyramid_vertex_buf = Arc::new(Buffer::create_from_slice(
        &device,
        vk::BufferUsageFlags::VERTEX_BUFFER,
        cast_slice(&pyramid_vertices),
    )?);

    let pyramid_index_buf = Arc::new(Buffer::create_from_slice(
        &device,
        vk::BufferUsageFlags::INDEX_BUFFER,
        cast_slice(&pyramid_indices),
    )?);

    // Create buffers for mode
    let model_vertex_buf = Arc::new(Buffer::create_from_slice(
        &device,
        vk::BufferUsageFlags::VERTEX_BUFFER,
        cast_slice(&model_vertices),
    )?);

    let model_index_buf = Arc::new(Buffer::create_from_slice(
        &device,
        vk::BufferUsageFlags::INDEX_BUFFER,
        cast_slice(&model_indices),
    )?);

    // Create render target image
    let width = 800u32;
    let height = 600u32;
    let color_image_size = (width * height * 4) as DeviceSize;

    let depth_image = Arc::new(Image::create(
        &device,
        ImageInfo::image_2d(
            width,
            height,
            vk::Format::D32_SFLOAT,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
        ),
    )?);

    // Load pre-compiled SPIR-V shaders
    let vert_spirv = include_bytes!("../dist/shaders/cube.vert.spv");
    let frag_spirv = include_bytes!("../dist/shaders/cube.frag.spv");

    // Create graphics pipeline with shaders
    let pipeline = Arc::new(GraphicPipeline::create(
        &device,
        GraphicPipelineInfo::default(),
        [
            Shader::new_vertex(vert_spirv.as_slice()),
            Shader::new_fragment(frag_spirv.as_slice()),
        ],
    )?);

    // Create transformation matrices using glam
    let aspect = width as f32 / height as f32;
    let projection = Mat4::perspective_rh(45.0_f32.to_radians(), aspect, 0.1, 100.0);

    let mut hash_pool = HashPool::new(&device);

    let (tx, rx) = std::sync::mpsc::sync_channel::<(usize, Arc<Buffer>, Lease<CommandBuffer>)>(QUEUE_CAPACITY);

    let handle = std::thread::spawn(move ||{
        while let Ok((frame_index, readback, mut cmd_lease)) = rx.recv() {
            println!("Received frame {}", frame_index);

            cmd_lease.wait_until_executed()
                .map_err(|e| println!("Failed to wait for command buffer execution: {}", e))?;

            println!("Frame read {}", frame_index);

            let start_at = std::time::Instant::now();
            let bytes: &[u8] = Buffer::mapped_slice(&readback);
            let bytes_vec: Vec<u8> = bytes.to_vec();
            let mapped_duration = start_at.elapsed();

            println!("Frame length: {}, mapped_duration: {}", bytes_vec.len(), mapped_duration.as_micros());

            // Save to raw file
            // std::fs::write(format!("frame_{:03}.raw", frame_index), &bytes)?;

            // Save to PNG
            image::save_buffer(
                format!("assets/cube_{:03}.png", frame_index),
                &bytes_vec.as_slice(),
                width,
                height,
                image::ColorType::Rgba8,
            )
                .map_err(|e| {
                    println!("Failed to save frame {}, e: {}", frame_index, e);
                })?;

            println!("✓ Saved frame {}, elapsed: {}us", frame_index, start_at.elapsed().as_micros());
        }

        Ok::<(), ()>(())
    });


    let mut frame_index = 0;
    loop {
        let t = frame_index as f32 / FRAME_COUNT as f32;

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

        let cube_push_constants = PushConstants {
            mvp: cube_mvp.to_cols_array_2d(),
        };

        let pyramid_push_constants = PushConstants {
            mvp: pyramid_mvp.to_cols_array_2d(),
        };

        let model_push_constants = PushConstants {
            mvp: model_mvp.to_cols_array_2d(),
        };

        let readback = Arc::new(Buffer::create(
            &device,
            BufferInfo::host_mem(
                color_image_size,
                vk::BufferUsageFlags::TRANSFER_DST,
            ),
        ).unwrap());

        // Create render graph
        let mut render_graph = RenderGraph::new();

        // Bind cube buffers
        let cube_vertex_node = render_graph.bind_node(&cube_vertex_buf);
        let cube_index_node = render_graph.bind_node(&cube_index_buf);

        // Bind pyramid buffers
        let pyramid_vertex_node = render_graph.bind_node(&pyramid_vertex_buf);
        let pyramid_index_node = render_graph.bind_node(&pyramid_index_buf);

        // Bind model buffers
        let model_vertex_node = render_graph.bind_node(&model_vertex_buf);
        let model_index_node = render_graph.bind_node(&model_index_buf);

        let color_image_info = ImageInfo::image_2d(
            width,
            height,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
        );

        let image_node = render_graph.bind_node(hash_pool.lease(color_image_info).unwrap());
        let depth_node = render_graph.bind_node(&depth_image);
        let readback_buf = render_graph.bind_node(readback.clone());

        render_graph
            .begin_pass("Render Cube")
            .bind_pipeline(&pipeline)
            .access_node(cube_vertex_node, AccessType::VertexBuffer)
            .access_node(cube_index_node, AccessType::IndexBuffer)
            .access_node(pyramid_vertex_node, AccessType::VertexBuffer)
            .access_node(pyramid_index_node, AccessType::IndexBuffer)
            .access_node(model_vertex_node, AccessType::VertexBuffer)
            .access_node(model_index_node, AccessType::IndexBuffer)
            .clear_color(0, image_node)
            .store_color(0, image_node)
            .clear_depth_stencil(depth_node)
            .record_subpass({
                let cube_indices_count = cube_indices.len() as u32;
                let pyramid_indices_count = pyramid_indices.len() as u32;
                let model_indices_count = model_indices.len() as u32;
                move |subpass, _| {
                    if true {
                        // Render cube
                        subpass.push_constants(cast_slice(&[cube_push_constants]));
                        subpass.bind_vertex_buffer(cube_vertex_node);
                        subpass.bind_index_buffer(cube_index_node, vk::IndexType::UINT16);
                        subpass.draw_indexed(cube_indices_count, 1, 0, 0, 0);

                        // Render pyramid
                        subpass.push_constants(cast_slice(&[pyramid_push_constants]));
                        subpass.bind_vertex_buffer(pyramid_vertex_node);
                        subpass.bind_index_buffer(pyramid_index_node, vk::IndexType::UINT16);
                        subpass.draw_indexed(pyramid_indices_count, 1, 0, 0, 0);
                    }
                    // Render model
                    subpass.push_constants(cast_slice(&[model_push_constants]));
                    subpass.bind_vertex_buffer(model_vertex_node);
                    subpass.bind_index_buffer(model_index_node, vk::IndexType::UINT16);
                    subpass.draw_indexed(model_indices_count, 1, 0, 0, 0);
                }
            });

        render_graph.copy_image_to_buffer(image_node, readback_buf);

        // let readback_buf = render_graph.unbind_node(readback_buf);
        // let image = render_graph.unbind_node(image_node);

        // --- Submit ---
        println!("Submitting frame {}", frame_index);
        let mut cmd_lease = render_graph
            .resolve()
            .submit(&mut hash_pool, 0, 0)?;

        //cmd_lease.wait_until_executed()?;

        println!("Sending frame {}", frame_index);
        tx.send((frame_index, readback, cmd_lease))?;

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
fn load_step_model_unfinished(path: &str) -> Result<(Vec<Vertex>, Vec<u16>), Box<dyn std::error::Error>> {
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
        println!("Processing shell {}/{}", shell_idx + 1, table.shell.len());

        // Convert shell to polygon mesh
        let Ok(shell) = table.to_compressed_shell(shell) else {
            println!("Failed to convert shell {} to polygon mesh", shell_id);
            continue
        };

        let mesh: PolygonMesh = shell.robust_triangulation(tol).to_polygon();
        let mesh_bounds = mesh.bounding_box();
        bounds = bounds.add(mesh_bounds);

        println!("  Vertices: {}, Faces: {}",
                 mesh.positions().len(),
                 mesh.faces().len());

        // Calculate a color based on shell index (for variety)
        let hue = (shell_idx as f32 * 0.618033988749895) % 1.0; // Golden ratio for distribution
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

    println!("Total vertices: {}, Total indices: {}",
             all_vertices.len(), all_indices.len());

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
