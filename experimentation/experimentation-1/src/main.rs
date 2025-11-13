use std::thread::sleep;
use std::time::{Duration, Instant};

fn main() {
    // --- Configuration parameters ---
    let step_distance_mm = 0.1_f32; // mm per step
    let travel_distance_mm = 10.0_f32; // target distance
    let travel_time_s = 1.0_f64; // desired time (seconds)
    let loop_hz = 2000.0_f64; // main loop frequency
    let min_pulse_us = 100; // minimum step pulse width in microseconds

    // --- Derived values ---
    let total_steps = (travel_distance_mm / step_distance_mm).round() as u32;
    let total_loops = (travel_time_s * loop_hz).round() as u32;
    let loop_period = Duration::from_secs_f64(1.0 / loop_hz);

    // Number of loops between steps (to evenly space steps)
    let loops_per_step = total_loops as f64 / total_steps as f64;

    println!("Total steps: {}", total_steps);
    println!("Total loops: {}", total_loops);
    println!("Loops per step: {:.3}", loops_per_step);

    // --- Motion control loop ---
    let mut step_accum = 0.0;
    let mut step_pin_state = false;
    let start_time = Instant::now();

    for loop_idx in 0..total_loops {
        let loop_start = Instant::now();

        // Accumulate fractional steps per loop
        step_accum += 1.0 / loops_per_step;

        // When accumulator >= 1.0, issue a step pulse
        if step_accum >= 1.0 {
            step_accum -= 1.0;

            // Generate a step pulse
            step_pin_state = true;
            println!(
                "Loop {:4}: STEP pulse HIGH for {} µs",
                loop_idx, min_pulse_us
            );

            // Simulate pulse duration
            sleep(Duration::from_micros(min_pulse_us as u64));

            // Step pulse LOW
            step_pin_state = false;
            println!("Loop {:4}: STEP pulse LOW", loop_idx);
        }

        // Maintain fixed loop frequency
        let elapsed = loop_start.elapsed();
        if elapsed < loop_period {
            sleep(loop_period - elapsed);
        }
    }

    let total_elapsed = start_time.elapsed().as_secs_f64();
    println!("Done. Total elapsed time: {:.3}s", total_elapsed);
}
