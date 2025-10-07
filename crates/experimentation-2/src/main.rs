use std::time::{Duration, Instant};

use plotters::prelude::*;
use rsruckig::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -------- Plotting --------------
    let mut time_data: Vec<f64> = Vec::new();
    let mut pos_data: Vec<f64> = Vec::new();
    let mut velocity_data: Vec<f64> = Vec::new();
    let mut acceleration_data: Vec<f64> = Vec::new();
    let mut step_data: Vec<f64> = Vec::new();
    let mut jerk_data: Vec<f64> = Vec::new();

    let mut step_time_points: Vec<f64> = Vec::new();
    let mut step_number_points: Vec<f64> = Vec::new();

    let mut time_s = 0.0;

    // -------- Configuration ---------
    let dt = 0.001; // 1 ms cycle (1000 Hz)
    let steps_per_mm = 32.0;
    let min_pulse_width_ns = 10_000; // 10 us

    // -------- Trajectory sequence ---------
    let trajectory: &[(f64, f64, f64, f64)] = &[
        // (position_mm, max_jerk, max_acc, max_vel)
        (100.0, 10.0, 50.0, 50.0),
        (50.0, 20.0, 25.0, 50.0),
        (300.0, 30.0, 75.0, 50.0),
    ];

    let mut ruckig = Ruckig::<1, ThrowErrorHandler>::new(None, dt);

    let mut input = InputParameter::<1>::new(None);
    let mut output = OutputParameter::<1>::new(None);
    let mut last_position = input.current_position.clone();

    let mut current_segment = 0;

    let mut step_accumulator: f64 = 0.0;
    let mut step_index: u64 = 0;

    // Store steps per move
    let mut steps_per_move: Vec<u32> = Vec::new();
    let mut total_steps_requested = 0u32;
    let mut total_steps_pulsed = 0u32;

    // Real-time loop reference
    let start_time = Instant::now();
    let mut cycle: u64 = 0;

    let mut prepare_next_segment = true;

    loop {
        if prepare_next_segment {
            prepare_next_segment = false;

            let (target_pos, max_jerk, max_acc, max_vel) = trajectory[current_segment];
            input.target_position = daov_stack![target_pos];
            input.target_velocity = daov_stack![0.0];
            input.target_acceleration = daov_stack![0.0];

            input.max_jerk = daov_stack![max_jerk];
            input.max_acceleration = daov_stack![max_acc];
            input.max_velocity = daov_stack![max_vel];

            output.time = 0.0;
            output.new_section = current_segment;
            ruckig.reset();

            let start_pos = input.current_position[0];
            let start_steps_i64 = (start_pos * steps_per_mm).round() as i64;
            let target_steps_i64 = (target_pos * steps_per_mm).round() as i64;
            let requested_steps = (target_steps_i64 - start_steps_i64).abs() as u32;

            steps_per_move.push(requested_steps);
            total_steps_requested = total_steps_requested.saturating_add(requested_steps);
        }

        println!(
            "Cycle: {}, Input: (position: {}, velocity: {}, acceleration: {}, target: {})",
            cycle,
            input.current_position[0],
            input.current_velocity[0],
            input.current_acceleration[0],
            input.target_position[0],
        );

        let result = ruckig
            .update(&input, &mut output)
            .unwrap();
        output.pass_to_input(&mut input);

        println!(
            "Result: {:?}, Output: (position: {}, velocity: {}, acceleration: {})",
            result, output.new_position[0], output.new_velocity[0], output.new_acceleration[0]
        );
        if matches!(result, RuckigResult::Finished) {
            // prepare for new segment
            current_segment += 1;
            if current_segment >= trajectory.len() {
                break;
            } else {
                prepare_next_segment = true;
            }
        }

        let new_pos = output.new_position[0];
        let delta_mm = (new_pos - last_position[0]).abs();

        // record data every cycle
        time_data.push(time_s);
        pos_data.push(new_pos);
        velocity_data.push(output.new_velocity[0]);
        acceleration_data.push(output.new_acceleration[0]);
        jerk_data.push(output.new_jerk[0]);
        step_data.push(step_index as f64);

        // Accumulate fractional steps
        step_accumulator += delta_mm * steps_per_mm;
        let n_steps = step_accumulator.floor() as u32;

        if n_steps > 0 {
            let cycle_start_ns = (cycle as f64 * dt * 1_000_000_000.0) as u64;
            let pulse_interval_ns = ((dt * 1_000_000_000.0) / n_steps as f64) as u64;

            for i in 0..n_steps {
                let pulse_start_ns = cycle_start_ns + i as u64 * pulse_interval_ns;

                sleep_until(start_time, pulse_start_ns);
                step_pin(true);
                sleep_until(start_time, pulse_start_ns + min_pulse_width_ns);
                step_pin(false);

                total_steps_pulsed += 1;

                // Record step edge for plotting (Option 3)
                let pulse_start_s = (cycle_start_ns + i as u64 * pulse_interval_ns) as f64 / 1e9;
                step_time_points.push(pulse_start_s);
                step_number_points.push((step_index as f64 + i as f64) / steps_per_mm); // scale to mm
            }

            step_accumulator -= n_steps as f64;
            step_index += n_steps as u64;
        }

        // Prepare input for next cycle
        last_position[0] = new_pos;
        cycle += 1;
        time_s += dt;

        // Sleep until next RT cycle
        let next_cycle_ns = (cycle as f64 * dt * 1_000_000_000.0) as u64;
        sleep_until(start_time, next_cycle_ns);
    }

    println!("Steps per move: {:?}", steps_per_move);
    println!("Total steps requested: {}", total_steps_requested);
    println!("Total steps pulsed: {}", total_steps_pulsed);

    assert_eq!(total_steps_requested, total_steps_pulsed, "Step count mismatch!");
    println!("All steps accounted for");

    // --- Plot trajectory + steps overlay ---
    //let root = BitMapBackend::new("trajectory.png", (3840, 2440)).into_drawing_area();
    let root = SVGBackend::new("trajectory.svg", (1000, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let all_x = time_data
        .iter()
        .chain(&step_time_points)
        .cloned()
        .collect::<Vec<f64>>();
    let x_min = all_x
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let x_max = all_x
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    // Find y-range across all datasets
    let all_y = pos_data
        .iter()
        .chain(&velocity_data)
        .chain(&acceleration_data)
        .chain(&jerk_data)
        .chain(&step_number_points)
        .cloned()
        .collect::<Vec<f64>>();
    let y_min = all_y
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let y_max = all_y
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);

    let mut chart = ChartBuilder::on(&root)
        .caption("Position (mm) vs Time (s) with Step Edges", ("sans-serif", 20))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(x_min..x_max, y_min..y_max)?;

    chart
        .configure_mesh()
        .x_desc("Time (s)")
        .y_desc("Position (mm)")
        .draw()?;

    chart
        .draw_series(LineSeries::new(
            time_data
                .iter()
                .cloned()
                .zip(pos_data.iter().cloned()),
            &RED,
        ))?
        .label("Position")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart
        .draw_series(LineSeries::new(
            time_data
                .iter()
                .cloned()
                .zip(velocity_data.iter().cloned()),
            &BLUE,
        ))?
        .label("Velocity")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

    chart
        .draw_series(LineSeries::new(
            time_data
                .iter()
                .cloned()
                .zip(acceleration_data.iter().cloned()),
            &GREEN,
        ))?
        .label("Acceleration")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &GREEN));

    chart
        .draw_series(LineSeries::new(
            time_data
                .iter()
                .cloned()
                .zip(jerk_data.iter().cloned()),
            &MAGENTA,
        ))?
        .label("Jerk")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &MAGENTA));

    // Overlay discrete step edges
    chart
        .draw_series(PointSeries::of_element(
            step_time_points
                .iter()
                .cloned()
                .zip(step_number_points.iter().cloned()),
            2,
            &BLACK,
            &|_coord, size, style| {
                return EmptyElement::at((_coord.0, _coord.1)) + Circle::new((0, 0), size, style.filled());
            },
        ))?
        .label("Step")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLACK));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;

    println!("Plot saved");

    Ok(())
}

// -------- GPIO / sleep helpers --------
fn step_pin(level: bool) {
    if level {
        // println!("STEP HIGH");
    } else {
        // println!("STEP LOW");
    }
}

fn sleep_until(start: Instant, target_ns: u64) {
    // let target_instant = start + Duration::from_nanos(target_ns);
    // while Instant::now() < target_instant {}
}
