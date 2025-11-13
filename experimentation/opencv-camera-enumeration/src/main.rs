use opencv::{Result, core, prelude::*, videoio};

fn main() -> Result<()> {
    // Try a reasonable range of camera indices
    for cam_idx in 0..10 {
        let mut cam = match videoio::VideoCapture::new(cam_idx, videoio::CAP_ANY) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if !videoio::VideoCapture::is_opened(&cam)? {
            continue;
        }

        println!("Camera {} opened successfully.", cam_idx);

        // Query current resolution and FPS
        let width = cam.get(videoio::CAP_PROP_FRAME_WIDTH)?;
        let height = cam.get(videoio::CAP_PROP_FRAME_HEIGHT)?;
        let fps = cam.get(videoio::CAP_PROP_FPS)?;
        let fourcc = cam.get(videoio::CAP_PROP_FOURCC)? as u32;
        let fourcc_str = format!(
            "{}{}{}{}",
            (fourcc & 0xFF) as u8 as char,
            ((fourcc >> 8) & 0xFF) as u8 as char,
            ((fourcc >> 16) & 0xFF) as u8 as char,
            ((fourcc >> 24) & 0xFF) as u8 as char
        );
        let settings = cam.get(videoio::CAP_PROP_SETTINGS)?;

        println!("  Default resolution: {}x{}", width, height);
        println!("  Default FPS: {}", fps);
        println!("  Default format (FOURCC): {}", fourcc_str);
        println!("  Default settings: {}", settings);

        // Try some common resolutions
        let common_resolutions = [
            (640, 480),
            (1280, 720),
            (1920, 1080),
            (2560, 1440),
            (3840, 2160),
        ];

        println!("  Testing common resolutions:");
        for &(w, h) in &common_resolutions {
            cam.set(videoio::CAP_PROP_FRAME_WIDTH, w as f64)?;
            cam.set(videoio::CAP_PROP_FRAME_HEIGHT, h as f64)?;

            let actual_w = cam.get(videoio::CAP_PROP_FRAME_WIDTH)?;
            let actual_h = cam.get(videoio::CAP_PROP_FRAME_HEIGHT)?;
            if (actual_w - w as f64).abs() < 1.0 && (actual_h - h as f64).abs() < 1.0 {
                println!("    Supported: {}x{}", w, h);
            } else {
                println!("    Not supported: {}x{}", w, h);
            }
        }

        // Optionally, test frame rates
        println!("  Testing FPS options:");
        for test_fps in [15.0, 30.0, 60.0, 120.0] {
            cam.set(videoio::CAP_PROP_FPS, test_fps)?;
            let actual_fps = cam.get(videoio::CAP_PROP_FPS)?;
            if (actual_fps - test_fps).abs() < 1.0 {
                println!("    Supported FPS: {}", test_fps);
            } else {
                println!("    Not supported FPS: {}", test_fps);
            }
        }

        println!();
    }

    Ok(())
}
