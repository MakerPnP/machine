use bytemuck::{cast_slice, Pod, Zeroable};
use glam::{Mat4, Vec3};
use screen_13::prelude::*;
use std::sync::Arc;
use screen_13::prelude::vk::DeviceSize;

use std::f32::consts::TAU;

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
            println!("Frame length: {}", bytes_vec.len());

            // Save to raw file
            // std::fs::write(format!("frame_{:03}.raw", frame_index), &bytes)?;
            let sum = bytes_vec.iter().fold(0, |acc, &x| acc + x as u32);
            let total_duration = start_at.elapsed();
            println!("Sum: {}, mapped_duration: {}, total_duration: {}us", sum, mapped_duration.as_micros(), total_duration.as_micros());

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

        let cube_push_constants = PushConstants {
            mvp: cube_mvp.to_cols_array_2d(),
        };

        let pyramid_push_constants = PushConstants {
            mvp: pyramid_mvp.to_cols_array_2d(),
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
            .clear_color(0, image_node)
            .store_color(0, image_node)
            .clear_depth_stencil(depth_node)
            .record_subpass(move |subpass, _| {
                // Render cube
                subpass.push_constants(cast_slice(&[cube_push_constants]));
                subpass.bind_vertex_buffer(cube_vertex_node);
                subpass.bind_index_buffer(cube_index_node, vk::IndexType::UINT16);
                subpass.draw_indexed(36, 1, 0, 0, 0);

                // Render pyramid
                subpass.push_constants(cast_slice(&[pyramid_push_constants]));
                subpass.bind_vertex_buffer(pyramid_vertex_node);
                subpass.bind_index_buffer(pyramid_index_node, vk::IndexType::UINT16);
                subpass.draw_indexed(18, 1, 0, 0, 0);
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