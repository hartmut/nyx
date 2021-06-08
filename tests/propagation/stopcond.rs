extern crate nalgebra as na;

extern crate nyx_space as nyx;

use nyx::celestia::{Bodies, Cosm, Orbit};
use nyx::dynamics::orbital::OrbitalDynamics;
use nyx::md::{Event, StateParameter};
use nyx::propagators::error_ctrl::RSSCartesianStep;
use nyx::propagators::{PropOpts, Propagator};
use nyx::time::{Epoch, J2000_OFFSET};
use nyx::TimeTagged;

#[test]
fn stop_cond_3rd_apo() {
    let cosm = Cosm::de438();
    let eme2k = cosm.frame("EME2000");

    let start_dt = Epoch::from_mjd_tai(J2000_OFFSET);
    let state = Orbit::cartesian(
        -2436.45, -2436.45, 6891.037, 5.088_611, -5.088_611, 0.0, start_dt, eme2k,
    );

    let period = state.period();

    // Track how many times we've passed by that TA again
    let apo_event = Event::apoapsis(); // Special event shortcut!

    let setup = Propagator::default(OrbitalDynamics::two_body());
    let mut prop = setup.with(state);
    // Propagate for at five orbital periods so we know we've passed the third one
    // NOTE: We start counting at ZERO, so finding the 3rd means grabbing the second found.
    let (third_apo, _) = prop.until_event(5 * period, &apo_event, 2).unwrap();

    println!("{}\t{}", start_dt + 2.0 * period, start_dt + 3.0 * period);
    // Confirm that this is the third apoapse event which is found
    assert!(
        (start_dt + 2.0 * period..start_dt + 3.0 * period).contains(&third_apo.dt),
        "converged on the wrong apoapse"
    );
    assert!(
        third_apo.dt - start_dt < 3.0 * period && third_apo.dt - start_dt >= 2.0 * period,
        "converged on the wrong apoapse"
    );
    assert!(
        (180.0 - third_apo.ta()).abs() < 1e-3,
        "converged, yet convergence critera not met"
    );
}

#[test]
fn stop_cond_3rd_peri() {
    let cosm = Cosm::de438();
    let eme2k = cosm.frame("EME2000");

    let start_dt = Epoch::from_mjd_tai(J2000_OFFSET);
    let state = Orbit::cartesian(
        -2436.45, -2436.45, 6891.037, 5.088_611, -5.088_611, 0.0, start_dt, eme2k,
    );

    let period = state.period();

    // Track how many times we've passed by that TA again
    let peri_event = Event::periapsis(); // Special event shortcut!

    let setup = Propagator::default(OrbitalDynamics::two_body());
    let mut prop = setup.with(state);
    // Propagate for at four orbital periods so we know we've passed the third one
    // NOTE: We're fetching the 3rd item because the initial state is actually at periapse,
    // which the event finder will find.
    let (third_peri, _) = prop.until_event(5 * period, &peri_event, 4).unwrap();

    println!("{:o}", third_peri);
    // Confirm that this is the third periapse event which is found
    // Again, the initial state is at periapse, so we don't check a N number of orbit forward.
    assert!(
        (start_dt + 1.9 * period..start_dt + 2.9 * period).contains(&third_peri.dt),
        "converged on the wrong apoapse"
    );
    assert!(
        third_peri.ta().abs() < 1e-1 || (360.0 - third_peri.ta().abs() < 1e-1),
        "converged, yet convergence critera not met"
    );
}

#[test]
fn stop_cond_nrho_apo() {
    use std::time::Instant;
    // The following test technically works, but the transformation of thousands of states
    // into another frame is quite slow...
    let cosm = Cosm::de438();
    let eme2k = cosm.frame("EME2000");
    let luna = cosm.frame("Luna");

    let dt = Epoch::from_gregorian_tai(2021, 5, 29, 19, 51, 16, 852_000);
    let state = Orbit::cartesian(
        166_473.631_302_239_7,
        -274_715.487_253_382_7,
        -211_233.210_176_686_7,
        0.933_451_604_520_018_4,
        0.436_775_046_841_900_9,
        -0.082_211_021_250_348_95,
        dt,
        eme2k,
    );

    let state_luna = cosm.frame_chg(&state, luna);
    println!(
        "Start state (dynamics: Earth, Moon, Sun gravity):\n{}",
        state_luna
    );

    let bodies = vec![Bodies::Earth, Bodies::Sun];
    let dynamics = OrbitalDynamics::point_masses(&bodies, cosm.clone());

    let setup = Propagator::rk89(
        dynamics,
        PropOpts::with_adaptive_step_s(1.0, 60.0, 1e-6, RSSCartesianStep {}),
    );

    let mut prop = setup.with(state);

    // NOTE: Here, we will propagate for the maximum duration in the original frame
    // Then convert that trajectory into the other frame, and perform the search there.
    // We can only do that for spacecraft and orbit trajectories since those have a frame.
    let prop_time = 4 * state_luna.period();
    let start = Instant::now();
    let (orbit, traj) = prop.for_duration_with_traj(prop_time).unwrap();

    let end_prop = Instant::now();
    println!(
        "Propagated for {} in {} ms:\n{:o}",
        prop_time,
        (end_prop - start).as_millis(),
        orbit,
    );

    // Create the event
    let apo_event = Event::apoapsis(); // Special event shortcut!

    // Convert this trajectory into the Luna frame
    let traj_luna = traj.to_frame(luna, cosm).unwrap();
    let end_conv = Instant::now();
    println!(
        "Converted EME2000 trajectory into Moon J2000 in {} ms",
        (end_conv - end_prop).as_millis()
    );

    // Now, find all of the requested events
    let events = traj_luna.find_all(&apo_event).unwrap();
    println!(
        "Found all {} events in {} ms",
        apo_event,
        (Instant::now() - end_conv).as_millis()
    );
    for event_state in &events {
        let delta_t = event_state.epoch() - dt;
        println!("{} after start:\n{:o}", delta_t, event_state);
        assert!((event_state.ta() - 180.0).abs() < 0.1);
    }
}

#[test]
fn line_of_nodes() {
    let cosm = Cosm::de438();
    let eme2k = cosm.frame("EME2000");

    let start_dt = Epoch::from_mjd_tai(J2000_OFFSET);
    let state = Orbit::cartesian(
        -2436.45, -2436.45, 6891.037, 5.088_611, -5.088_611, 0.0, start_dt, eme2k,
    );

    let period = state.period();

    let lon_event = Event::new(StateParameter::GeodeticLongitude, 0.0);

    let setup = Propagator::default(OrbitalDynamics::two_body());
    let mut prop = setup.with(state);
    let (lon_state, _) = prop.until_event(3 * period, &lon_event, 0).unwrap();
    println!(
        "{:o} => longitude = {} degrees",
        lon_state,
        lon_state.geodetic_longitude()
    );

    assert!(
        lon_state.geodetic_longitude().abs() < lon_event.value_precision,
        "converged, yet convergence critera not met"
    );
}

#[test]
fn latitude() {
    let cosm = Cosm::de438();
    let eme2k = cosm.frame("EME2000");

    let start_dt = Epoch::from_mjd_tai(J2000_OFFSET);
    let state = Orbit::cartesian(
        -2436.45, -2436.45, 6891.037, 5.088_611, -5.088_611, 0.0, start_dt, eme2k,
    );

    let period = state.period();

    let lat_event = Event::new(StateParameter::GeodeticLatitude, 2.0);

    let setup = Propagator::default(OrbitalDynamics::two_body());
    let mut prop = setup.with(state);
    let (lon_state, _) = prop.until_event(3 * period, &lat_event, 0).unwrap();
    println!(
        "{:o} => latitude = {} degrees",
        lon_state,
        lon_state.geodetic_latitude()
    );

    assert!(
        (2.0 - lon_state.geodetic_latitude()).abs() < lat_event.value_precision,
        "converged, yet convergence critera not met"
    );
}