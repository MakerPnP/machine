pub(crate) fn hsv_to_rgb(h: f32, s: f32, v: f32) -> u32 {
    let c = v * s;
    let hh = h * 6.0;
    let x = c * (1.0 - ((hh % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match hh as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    let r = ((r + m) * 255.0) as u32;
    let g = ((g + m) * 255.0) as u32;
    let b = ((b + m) * 255.0) as u32;

    (r << 16) | (g << 8) | b
}

pub fn rainbow_wave(rgb_leds: &mut [u32], t: u32) {
    for (i, value) in rgb_leds.iter_mut().enumerate() {
        // phase offset per LED + time
        let h = ((t as f32 * 0.01) + (i as f32 * 0.25)) % 1.0;

        // full saturation, decent brightness
        *value = hsv_to_rgb(h, 1.0, 0.5);
    }
}