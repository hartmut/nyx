extern crate hifitime;
extern crate nalgebra as na;
#[macro_use]
extern crate approx; // For the macro relative_eq!

extern crate nyx_space as nyx;

use nyx::utils::rss_state_errors;

#[test]
fn two_body_custom() {
    use hifitime::{Epoch, J2000_OFFSET};
    use na::Vector6;
    use nyx::celestia::{bodies, Cosm, Geoid, State};
    use nyx::dynamics::celestial::CelestialDynamics;
    use nyx::propagators::error_ctrl::RSSStepPV;
    use nyx::propagators::*;

    let prop_time = 24.0 * 3_600.0;

    let cosm = Cosm::from_xb("./de438s");
    let earth_geoid = cosm.geoid_from_id(bodies::EARTH_BARYCENTER);

    let dt = Epoch::from_mjd_tai(J2000_OFFSET);
    let mut state = State::<Geoid>::from_cartesian(-2436.45, -2436.45, 6891.037, 5.088_611, -5.088_611, 0.0, dt, earth_geoid);
    state.frame.gm = 398_600.441_5;

    let rslt = Vector6::new(
        -5_971.194_191_684_025,
        3_945.506_653_624_713,
        2_864.636_617_867_305,
        0.049_096_957_141_074_773,
        -4.185_093_318_149_709,
        5.848_940_867_979_16,
    );

    let mut dynamics = CelestialDynamics::two_body(state);

    let mut prop = Propagator::new::<RK89>(&mut dynamics, &PropOpts::<RSSStepPV>::default());
    prop.until_time_elapsed(prop_time);
    assert_eq!(prop.state(), rslt, "two body prop failed");
    // And now do the backprop
    prop.until_time_elapsed(-prop_time);
    let (err_r, err_v) = rss_state_errors(&prop.state(), &state.to_cartesian_vec());
    assert!(
        err_r < 1e-5,
        "two body back prop failed to return to the initial state in position"
    );
    assert!(
        err_v < 1e-8,
        "two body back prop failed to return to the initial state in velocity"
    );
}

#[test]
fn two_body_dynamics() {
    use hifitime::{Epoch, J2000_OFFSET};
    use na::Vector6;
    use nyx::celestia::{bodies, Cosm, Geoid, State};
    use nyx::dynamics::celestial::CelestialDynamics;
    use nyx::propagators::error_ctrl::RSSStepPV;
    use nyx::propagators::*;
    use std::f64::EPSILON;

    let prop_time = 24.0 * 3_600.0;

    let cosm = Cosm::from_xb("./de438s");
    let earth_geoid = cosm.geoid_from_id(bodies::EARTH_BARYCENTER);

    let dt = Epoch::from_mjd_tai(J2000_OFFSET);
    let state = State::<Geoid>::from_cartesian(-2436.45, -2436.45, 6891.037, 5.088_611, -5.088_611, 0.0, dt, earth_geoid);

    let rslt = Vector6::new(
        -5_971.194_376_797_643,
        3_945.517_912_574_178_4,
        2_864.620_957_744_429_2,
        0.049_083_101_605_507_95,
        -4.185_084_125_817_658,
        5.848_947_462_472_877,
    );

    let mut dynamics = CelestialDynamics::two_body(state);

    let mut prop = Propagator::new::<RK89>(&mut dynamics, &PropOpts::<RSSStepPV>::default());
    prop.until_time_elapsed(prop_time);
    assert!((prop.dynamics.state.dt.as_mjd_tai_days() - dt.as_mjd_tai_days() - 1.0).abs() <= EPSILON);
    assert!(abs_diff_eq!(prop.state(), rslt, epsilon = 2e-9f64), "two body prop failed");
    // And now do the backprop
    prop.until_time_elapsed(-prop_time);
    let (err_r, err_v) = rss_state_errors(&prop.state(), &state.to_cartesian_vec());
    assert!(
        err_r < 1e-5,
        "two body back prop failed to return to the initial state in position"
    );
    assert!(
        err_v < 1e-8,
        "two body back prop failed to return to the initial state in velocity"
    );
    assert!((prop.dynamics.state.dt.as_mjd_tai_days() - dt.as_mjd_tai_days()).abs() <= EPSILON);
    // Forward propagation again to confirm that we can do repeated calls
    prop.until_time_elapsed(prop_time);
    assert!((prop.dynamics.state.dt.as_mjd_tai_days() - dt.as_mjd_tai_days() - 1.0).abs() <= EPSILON);
    let (err_r, err_v) = rss_state_errors(&prop.state(), &rslt);
    assert!(
        err_r < 1e-5,
        "two body back+fwd prop failed to return to the initial state in position"
    );
    assert!(
        err_v < 1e-8,
        "two body back+fwd prop failed to return to the initial state in velocity"
    );
}

#[test]
fn halo_earth_moon_dynamics() {
    /*
    We validate against GMAT after switching the GMAT script to use de438s.bsp. We are using GMAT's default GM values.
    The state in `rslt` is exactly the GMAT output.
    */
    use hifitime::Epoch;
    use na::Vector6;
    use nyx::celestia::{bodies, Cosm, Geoid, State};
    use nyx::dynamics::celestial::CelestialDynamics;
    use nyx::propagators::error_ctrl::RSSStatePV;
    use nyx::propagators::{PropOpts, Propagator, RK89};

    let prop_time = 24.0 * 3_600.0;

    let mut cosm = Cosm::from_xb("./de438s");
    // Modify GMs to match GMAT's
    cosm.mut_gm_for_geoid_id(bodies::EARTH, 398_600.441_5);
    cosm.mut_gm_for_geoid_id(bodies::EARTH_MOON, 4_902.800_582_147_8);
    let earth = cosm.geoid_from_id(bodies::EARTH);

    let start_time = Epoch::from_gregorian_tai_at_midnight(2020, 1, 1);

    let halo_rcvr = State::<Geoid>::from_cartesian(
        333_321.004_516,
        -76_134.198_887,
        -20_873.831_939,
        0.257_153_712,
        0.930_284_066,
        0.346_177,
        start_time,
        earth,
    );

    // GMAT data
    let rslt = Vector6::new(
        345_395.216_758_754_4,
        5_967.890_264_751_025,
        7_350.734_617_702_599,
        0.022_370_754_768_832_33,
        0.957_450_818_399_485_1,
        0.303_172_019_604_272_5,
    );

    let bodies = vec![bodies::EARTH_MOON];
    let mut dynamics = CelestialDynamics::new(halo_rcvr, bodies, &cosm);

    let mut prop = Propagator::new::<RK89>(&mut dynamics, &PropOpts::with_fixed_step(10.0, RSSStatePV {}));
    prop.until_time_elapsed(prop_time);
    let (err_r, err_v) = rss_state_errors(&prop.state(), &rslt);

    println!("Absolute errors");
    let delta = prop.state() - rslt;
    for i in 0..6 {
        print!("{:.0e}\t", delta[i].abs());
    }
    println!();

    println!(
        "RSS errors:\tpos = {:.5e} km\tvel = {:.5e} km/s\ninit\t{}\nfinal\t{}",
        err_r, err_v, halo_rcvr, prop.dynamics.state
    );

    assert!(err_r < 1e-5, format!("multi body failed in position: {:.5e}", err_r));
    assert!(err_v < 1e-10, format!("multi body failed in velocity: {:.5e}", err_v));
}

#[test]
fn halo_earth_moon_dynamics_adaptive() {
    /*
    We validate against GMAT after switching the GMAT script to use de438s.bsp. We are using GMAT's default GM values.
    The state in `rslt` is exactly the GMAT output.
    */
    use hifitime::Epoch;
    use na::Vector6;
    use nyx::celestia::{bodies, Cosm, Geoid, State};
    use nyx::dynamics::celestial::CelestialDynamics;
    use nyx::propagators::{PropOpts, Propagator, RK89};

    let prop_time = 24.0 * 3_600.0;

    let mut cosm = Cosm::from_xb("./de438s");
    // Modify GMs to match GMAT's
    cosm.mut_gm_for_geoid_id(bodies::EARTH, 398_600.441_5);
    cosm.mut_gm_for_geoid_id(bodies::EARTH_MOON, 4_902.800_582_147_8);
    let earth = cosm.geoid_from_id(bodies::EARTH);

    let start_time = Epoch::from_gregorian_tai_at_midnight(2002, 2, 7);

    let halo_rcvr = State::<Geoid>::from_cartesian(
        333_321.004_516,
        -76_134.198_887,
        -20_873.831_939,
        0.257_153_712,
        0.930_284_066,
        0.346_177,
        start_time,
        earth,
    );

    let rslt = Vector6::new(
        343016.0281933062,
        6118.870782679712,
        9463.253311291081,
        -0.03388550441829203,
        0.9619425779605422,
        0.3517381217093635,
    );

    let bodies = vec![bodies::EARTH_MOON];
    let mut dynamics = CelestialDynamics::new(halo_rcvr, bodies, &cosm);

    let mut prop = Propagator::new::<RK89>(&mut dynamics, &PropOpts::default());
    prop.until_time_elapsed(prop_time);
    let (err_r, err_v) = rss_state_errors(&prop.state(), &rslt);

    println!("Absolute errors");
    let delta = prop.state() - rslt;
    for i in 0..6 {
        print!("{:.0e}\t", delta[i].abs());
    }
    println!();

    println!(
        "RSS errors:\tpos = {:.5e} km\tvel = {:.5e} km/s\ninit\t{}\nfinal\t{}",
        err_r, err_v, halo_rcvr, prop.dynamics.state
    );

    assert!(err_r < 1e-6, format!("multi body failed in position: {:.5e}", err_r));
    assert!(err_v < 1e-11, format!("multi body failed in velocity: {:.5e}", err_v));
}

#[test]
fn llo_earth_moon_dynamics_adaptive() {
    /*
    We validate against GMAT after switching the GMAT script to use de438s.bsp. We are using GMAT's default GM values.
    The state in `rslt` is exactly the GMAT output.
    */
    use hifitime::Epoch;
    use na::Vector6;
    use nyx::celestia::{bodies, Cosm, Geoid, State};
    use nyx::dynamics::celestial::CelestialDynamics;
    use nyx::propagators::{PropOpts, Propagator, RK89};

    let prop_time = 24.0 * 3_600.0;

    let mut cosm = Cosm::from_xb("./de438s");
    // Modify GMs to match GMAT's
    cosm.mut_gm_for_geoid_id(bodies::EARTH, 398_600.441_5);
    cosm.mut_gm_for_geoid_id(bodies::EARTH_MOON, 4_902.800_582_147_8);
    let earth = cosm.geoid_from_id(bodies::EARTH);

    let start_time = Epoch::from_gregorian_tai_at_midnight(2002, 2, 7);

    let llo_xmtr = State::<Geoid>::from_cartesian(
        3.919_869_89e5,
        -7.493_039_70e4,
        -7.022_605_11e4,
        -6.802_604_18e-1,
        1.992_053_61,
        4.369_389_94e-1,
        start_time,
        earth,
    );

    // GMAT data
    let rslt = Vector6::new(
        322883.8868354332,
        97580.280858158,
        -30871.08580743158,
        -0.9340396297270035,
        1.980106615205608,
        0.4726308955048544,
    );

    let bodies = vec![bodies::EARTH_MOON];
    let mut dynamics = CelestialDynamics::new(llo_xmtr, bodies, &cosm);

    let mut prop = Propagator::new::<RK89>(&mut dynamics, &PropOpts::default());
    prop.until_time_elapsed(prop_time);
    let (err_r, err_v) = rss_state_errors(&prop.state(), &rslt);

    println!("Absolute errors");
    let delta = prop.state() - rslt;
    for i in 0..6 {
        print!("{:.0e}\t", delta[i].abs());
    }
    println!();

    println!(
        "RSS errors:\tpos = {:.5e} km\tvel = {:.5e} km/s\ninit\t{}\nfinal\t{}",
        err_r, err_v, llo_xmtr, prop.dynamics.state
    );

    assert!(err_r < 1e-5, format!("multi body failed in position: {:.5e}", err_r));
    assert!(err_v < 1e-8, format!("multi body failed in velocity: {:.5e}", err_v));
}

#[test]
fn halo_multi_body_dynamics() {
    /*
    We validate against GMAT after switching the GMAT script to use de438s.bsp. We are using GMAT's default GM values.
    The state in `rslt` is exactly the GMAT output.
    */
    use hifitime::Epoch;
    use na::Vector6;
    use nyx::celestia::{bodies, Cosm, Geoid, State};
    use nyx::dynamics::celestial::CelestialDynamics;
    use nyx::propagators::error_ctrl::RSSStatePV;
    use nyx::propagators::{PropOpts, Propagator, RK89};

    let prop_time = 24.0 * 3_600.0;

    let mut cosm = Cosm::from_xb("./de438s");
    // Modify GMs to match GMAT's
    cosm.mut_gm_for_geoid_id(bodies::EARTH, 398_600.441_5);
    cosm.mut_gm_for_geoid_id(bodies::EARTH_MOON, 4_902.800_582_147_8);
    cosm.mut_gm_for_geoid_id(bodies::JUPITER_BARYCENTER, 126_712_767.857_80);
    cosm.mut_gm_for_geoid_id(bodies::SUN, 132_712_440_017.99);
    let earth = cosm.geoid_from_id(bodies::EARTH);

    let start_time = Epoch::from_gregorian_tai_at_midnight(2020, 1, 1);

    let halo_rcvr = State::<Geoid>::from_cartesian(
        333_321.004_516,
        -76_134.198_887,
        -20_873.831_939,
        0.257_153_712,
        0.930_284_066,
        0.346_177,
        start_time,
        earth,
    );

    // GMAT data
    let rslt = Vector6::new(
        345_350.664_306_402_7,
        5_930.672_402_473_843,
        7_333.283_870_811_47,
        0.021_298_196_465_430_16,
        0.956_678_964_966_812_2,
        0.302_817_582_487_008_6,
    );

    let bodies = vec![bodies::EARTH_MOON, bodies::SUN, bodies::JUPITER_BARYCENTER];
    let mut dynamics = CelestialDynamics::new(halo_rcvr, bodies, &cosm);

    let mut prop = Propagator::new::<RK89>(&mut dynamics, &PropOpts::with_fixed_step(10.0, RSSStatePV {}));
    prop.until_time_elapsed(prop_time);
    let (err_r, err_v) = rss_state_errors(&prop.state(), &rslt);

    println!("Absolute errors");
    let delta = prop.state() - rslt;
    for i in 0..6 {
        print!("{:.0e}\t", delta[i].abs());
    }
    println!();

    println!(
        "RSS errors:\tpos = {:.5e} km\tvel = {:.5e} km/s\ninit\t{}\nfinal\t{}",
        err_r, err_v, halo_rcvr, prop.dynamics.state
    );

    assert!(err_r < 1e-5, format!("multi body failed in position: {:.5e}", err_r));
    assert!(err_v < 1e-10, format!("multi body failed in velocity: {:.5e}", err_v));
}

#[test]
fn halo_multi_body_dynamics_adaptive() {
    /*
    We validate against GMAT after switching the GMAT script to use de438s.bsp. We are using GMAT's default GM values.
    The state in `rslt` is exactly the GMAT output.
    */
    use hifitime::Epoch;
    use na::Vector6;
    use nyx::celestia::{bodies, Cosm, Geoid, State};
    use nyx::dynamics::celestial::CelestialDynamics;
    use nyx::propagators::{PropOpts, Propagator, RK89};

    let prop_time = 24.0 * 3_600.0;

    let mut cosm = Cosm::from_xb("./de438s");
    // Modify GMs to match GMAT's
    cosm.mut_gm_for_geoid_id(bodies::EARTH, 398_600.441_5);
    cosm.mut_gm_for_geoid_id(bodies::EARTH_MOON, 4_902.800_582_147_8);
    cosm.mut_gm_for_geoid_id(bodies::JUPITER_BARYCENTER, 126_712_767.857_80);
    cosm.mut_gm_for_geoid_id(bodies::SUN, 132_712_440_017.99);
    let earth = cosm.geoid_from_id(bodies::EARTH);

    // let start_time = Epoch::from_gregorian_tai_at_midnight(2020, 1, 1);
    let start_time = Epoch::from_gregorian_tai_at_midnight(2002, 2, 7);

    let halo_rcvr = State::<Geoid>::from_cartesian(
        333_321.004_516,
        -76_134.198_887,
        -20_873.831_939,
        0.257_153_712,
        0.930_284_066,
        0.346_177,
        start_time,
        earth,
    );

    // GMAT data
    let rslt = Vector6::new(
        343063.3150797269,
        6045.912866799058,
        9430.044002816507,
        -0.03284104050047527,
        0.9602726135306772,
        0.3509814313220894,
    );

    let bodies = vec![bodies::EARTH_MOON, bodies::SUN, bodies::JUPITER_BARYCENTER];
    let mut dynamics = CelestialDynamics::new(halo_rcvr, bodies, &cosm);

    let mut prop = Propagator::new::<RK89>(&mut dynamics, &PropOpts::default());
    prop.until_time_elapsed(prop_time);
    let (err_r, err_v) = rss_state_errors(&prop.state(), &rslt);

    println!("Absolute errors");
    let delta = prop.state() - rslt;
    for i in 0..6 {
        print!("{:.0e}\t", delta[i].abs());
    }
    println!();

    println!(
        "RSS errors:\tpos = {:.5e} km\tvel = {:.5e} km/s\ninit\t{}\nfinal\t{}",
        err_r, err_v, halo_rcvr, prop.dynamics.state
    );

    assert!(err_r < 1e-6, format!("multi body failed in position: {:.5e}", err_r));
    assert!(err_v < 1e-11, format!("multi body failed in velocity: {:.5e}", err_v));
}

#[test]
fn llo_multi_body_dynamics_adaptive() {
    /*
    We validate against GMAT after switching the GMAT script to use de438s.bsp. We are using GMAT's default GM values.
    The state in `rslt` is exactly the GMAT output.
    */
    use hifitime::Epoch;
    use na::Vector6;
    use nyx::celestia::{bodies, Cosm, Geoid, State};
    use nyx::dynamics::celestial::CelestialDynamics;
    use nyx::propagators::{PropOpts, Propagator, RK89};

    let prop_time = 24.0 * 3_600.0;

    let mut cosm = Cosm::from_xb("./de438s");
    // Modify GMs to match GMAT's
    cosm.mut_gm_for_geoid_id(bodies::EARTH, 398_600.441_5);
    cosm.mut_gm_for_geoid_id(bodies::EARTH_MOON, 4_902.800_582_147_8);
    cosm.mut_gm_for_geoid_id(bodies::JUPITER_BARYCENTER, 126_712_767.857_80);
    cosm.mut_gm_for_geoid_id(bodies::SUN, 132_712_440_017.99);
    let earth = cosm.geoid_from_id(bodies::EARTH);

    let start_time = Epoch::from_gregorian_tai_at_midnight(2002, 2, 7);

    let llo_xmtr = State::<Geoid>::from_cartesian(
        3.919_869_89e5,
        -7.493_039_70e4,
        -7.022_605_11e4,
        -6.802_604_18e-1,
        1.992_053_61,
        4.369_389_94e-1,
        start_time,
        earth,
    );

    // GMAT data
    let rslt = Vector6::new(
        322931.8517607412,
        97497.69973881113,
        -30899.3238203672,
        -0.9330952021437368,
        1.978291140770421,
        0.4720361979683693,
    );

    let bodies = vec![bodies::EARTH_MOON, bodies::SUN, bodies::JUPITER_BARYCENTER];
    let mut dynamics = CelestialDynamics::new(llo_xmtr, bodies, &cosm);

    let mut prop = Propagator::new::<RK89>(&mut dynamics, &PropOpts::default());
    prop.until_time_elapsed(prop_time);
    let (err_r, err_v) = rss_state_errors(&prop.state(), &rslt);

    println!("Absolute errors");
    let delta = prop.state() - rslt;
    for i in 0..6 {
        print!("{:.0e}\t", delta[i].abs());
    }
    println!();

    println!(
        "RSS errors:\tpos = {:.5e} km\tvel = {:.5e} km/s\ninit\t{}\nfinal\t{}",
        err_r, err_v, llo_xmtr, prop.dynamics.state
    );

    assert!(err_r < 2e-6, format!("multi body failed in position: {:.5e}", err_r));
    assert!(err_v < 1e-9, format!("multi body failed in velocity: {:.5e}", err_v));
}

#[test]
fn leo_multi_body_dynamics_adaptive_wo_moon() {
    /*
    We validate against GMAT after switching the GMAT script to use de438s.bsp. We are using GMAT's default GM values.
    The state in `rslt` is exactly the GMAT output.
    */
    use hifitime::Epoch;
    use na::Vector6;
    use nyx::celestia::{bodies, Cosm, Geoid, State};
    use nyx::dynamics::celestial::CelestialDynamics;
    use nyx::propagators::{PropOpts, Propagator, RK89};

    let prop_time = 24.0 * 3_600.0;

    let mut cosm = Cosm::from_xb("./de438s");
    // Modify GMs to match GMAT's
    cosm.mut_gm_for_geoid_id(bodies::EARTH, 398_600.441_5);
    cosm.mut_gm_for_geoid_id(bodies::EARTH_MOON, 4_902.800_582_147_8);
    cosm.mut_gm_for_geoid_id(bodies::JUPITER_BARYCENTER, 126_712_767.857_80);
    cosm.mut_gm_for_geoid_id(bodies::SUN, 132_712_440_017.99);
    let earth = cosm.geoid_from_id(bodies::EARTH);

    let start_time = Epoch::from_gregorian_tai_at_midnight(2020, 1, 1);

    let leo = State::<Geoid>::from_cartesian(-2436.45, -2436.45, 6891.037, 5.088_611, -5.088_611, 0.0, start_time, earth);

    // GMAT data
    let rslt = Vector6::new(
        -5_971.190_141_842_914,
        3_945.572_972_028_369,
        2_864.554_642_502_679,
        0.049_014_376_371_383_95,
        -4.185_051_832_316_421,
        5.848_971_837_743_221,
    );

    let bodies = vec![bodies::EARTH_MOON, bodies::SUN, bodies::JUPITER_BARYCENTER];
    let mut dynamics = CelestialDynamics::new(leo, bodies, &cosm);

    let mut prop = Propagator::new::<RK89>(&mut dynamics, &PropOpts::default());
    prop.until_time_elapsed(prop_time);
    let (err_r, err_v) = rss_state_errors(&prop.state(), &rslt);

    println!("Absolute errors");
    let delta = prop.state() - rslt;
    for i in 0..6 {
        print!("{:.0e}\t", delta[i].abs());
    }
    println!();

    println!(
        "RSS errors:\tpos = {:.5e} km\tvel = {:.5e} km/s\ninit\t{}\nfinal\t{}",
        err_r, err_v, leo, prop.dynamics.state
    );

    assert!(err_r < 5e-7, format!("multi body failed in position: {:.5e}", err_r));
    assert!(err_v < 5e-10, format!("multi body failed in velocity: {:.5e}", err_v));
}

#[test]
fn leo_multi_body_dynamics_adaptive() {
    /*
    We validate against GMAT after switching the GMAT script to use de438s.bsp. We are using GMAT's default GM values.
    The state in `rslt` is exactly the GMAT output.
    */
    use hifitime::Epoch;
    use na::Vector6;
    use nyx::celestia::{bodies, Cosm, Geoid, State};
    use nyx::dynamics::celestial::CelestialDynamics;
    use nyx::propagators::{PropOpts, Propagator, RK89};

    let prop_time = 24.0 * 3_600.0;

    let mut cosm = Cosm::from_xb("./de438s");
    // Modify GMs to match GMAT's
    cosm.mut_gm_for_geoid_id(bodies::EARTH, 398_600.441_5);
    cosm.mut_gm_for_geoid_id(bodies::JUPITER_BARYCENTER, 126_712_767.857_80);
    cosm.mut_gm_for_geoid_id(bodies::SUN, 132_712_440_017.99);
    let earth = cosm.geoid_from_id(bodies::EARTH);

    let start_time = Epoch::from_gregorian_tai_at_midnight(2020, 1, 1);

    let leo = State::<Geoid>::from_cartesian(-2436.45, -2436.45, 6891.037, 5.088_611, -5.088_611, 0.0, start_time, earth);

    // GMAT data
    let rslt = Vector6::new(
        -5_971.190_491_039_24,
        3_945.529_211_711_111,
        2_864.613_171_213_388,
        0.049_086_325_111_121_92,
        -4.185_065_854_096_239,
        5.848_960_991_136_447,
    );

    let bodies = vec![bodies::SUN, bodies::JUPITER_BARYCENTER];
    let mut dynamics = CelestialDynamics::new(leo, bodies, &cosm);

    let mut prop = Propagator::new::<RK89>(&mut dynamics, &PropOpts::default());
    prop.until_time_elapsed(prop_time);
    let (err_r, err_v) = rss_state_errors(&prop.state(), &rslt);

    println!("Absolute errors");
    let delta = prop.state() - rslt;
    for i in 0..6 {
        print!("{:.0e}\t", delta[i].abs());
    }
    println!();

    println!(
        "RSS errors:\tpos = {:.5e} km\tvel = {:.5e} km/s\ninit\t{}\nfinal\t{}",
        err_r, err_v, leo, prop.dynamics.state
    );

    assert!(err_r < 3e-6, format!("multi body failed in position: {:.5e}", err_r));
    assert!(err_v < 3e-9, format!("multi body failed in velocity: {:.5e}", err_v));
}

#[test]
fn two_body_dual() {
    // This is a duplicate of the differentials test in hyperdual.
    extern crate nalgebra as na;
    use self::na::{Matrix6, Vector6, U3};
    use hifitime::Epoch;
    use nyx::celestia::{Cosm, Geoid, State};
    use nyx::dynamics::celestial::CelestialDynamicsStm;
    use nyx::od::AutoDiffDynamics;
    use nyx::propagators::error_ctrl::RSSStatePV;
    use nyx::propagators::*;

    let cosm = Cosm::from_xb("./de438s");
    let earth_geoid = cosm.geoid_from_id(3);

    let init = State::<Geoid>::from_cartesian(
        -9_042.862_233_600_335,
        18_536.333_069_123_244,
        6_999.957_069_486_411_5,
        -3.288_789_003_770_57,
        -2.226_285_193_102_822,
        1.646_738_380_722_676_5,
        Epoch::from_mjd_tai(21_546.0),
        earth_geoid,
    );

    let expected_fx = Vector6::new(
        -3.288_789_003_770_57,
        -2.226_285_193_102_822,
        1.646_738_380_722_676_5,
        0.000_348_875_166_673_120_14,
        -0.000_715_134_890_031_838_4,
        -0.000_270_059_537_150_490_5,
    );

    let mut dynamics = CelestialDynamicsStm::two_body(init);
    let (fx, grad) = dynamics.compute(0.0, &init.to_cartesian_vec());

    assert!(
        (fx - expected_fx).norm() < 1e-16,
        "f(x) computation is incorrect {:e}",
        (fx - expected_fx).norm()
    );

    let mut expected = Matrix6::zeros();

    expected[(0, 3)] = 1.0;
    expected[(1, 4)] = 1.0;
    expected[(2, 5)] = 1.0;
    expected[(3, 0)] = -0.000_000_018_628_398_391_083_86;
    expected[(4, 0)] = -0.000_000_040_897_747_124_379_53;
    expected[(5, 0)] = -0.000_000_015_444_396_313_003_294;
    expected[(3, 1)] = -0.000_000_040_897_747_124_379_53;
    expected[(4, 1)] = 0.000_000_045_253_271_058_430_05;
    expected[(5, 1)] = 0.000_000_031_658_391_636_846_51;
    expected[(3, 2)] = -0.000_000_015_444_396_313_003_294;
    expected[(4, 2)] = 0.000_000_031_658_391_636_846_51;
    expected[(5, 2)] = -0.000_000_026_624_872_667_346_21;

    assert!(
        (grad - expected).norm() < 1e-16,
        "gradient computation is incorrect {:e}",
        (grad - expected).norm()
    );

    assert_eq!(dynamics.state, init);

    let prop_time = 24.0 * 3_600.0;

    let mut prop = Propagator::new::<RK89>(&mut dynamics, &PropOpts::with_fixed_step(10.0, RSSStatePV {}));
    prop.until_time_elapsed(prop_time);

    // Check that the STM is correct by back propagating by the previous step, and multiplying by the STM.
    let final_state = prop.dynamics.state.to_cartesian_vec();
    let final_stm = prop.dynamics.stm;
    let final_step = prop.latest_details().step;
    prop.until_time_elapsed(-final_step);

    // And check the difference
    let stm_err = final_stm * prop.dynamics.state.to_cartesian_vec() - final_state;
    let radius_err = stm_err.fixed_rows::<U3>(0).into_owned();
    let velocity_err = stm_err.fixed_rows::<U3>(3).into_owned();

    assert!(radius_err.norm() < 1e-1);
    assert!(velocity_err.norm() < 1e-1);
}

#[test]
fn multi_body_dynamics_dual() {
    use hifitime::Epoch;
    use na::U3;
    use nyx::celestia::{bodies, Cosm, Geoid, State};
    use nyx::dynamics::celestial::CelestialDynamicsStm;
    use nyx::propagators::error_ctrl::RSSStatePV;
    use nyx::propagators::*;

    let prop_time = 24.0 * 3_600.0;

    let cosm = Cosm::from_xb("./de438s");
    let earth_geoid = cosm.geoid_from_id(bodies::EARTH);

    let start_time = Epoch::from_gregorian_tai_at_midnight(2020, 1, 1);

    let halo_rcvr = State::<Geoid>::from_cartesian(
        333_321.004_516,
        -76_134.198_887,
        -20_873.831_939,
        0.257_153_712,
        0.930_284_066,
        0.346_177,
        start_time,
        earth_geoid,
    );

    let bodies = vec![bodies::EARTH_MOON, bodies::SUN, bodies::JUPITER_BARYCENTER];
    let mut dynamics = CelestialDynamicsStm::new(halo_rcvr, bodies, &cosm);

    let mut prop = Propagator::new::<RK89>(&mut dynamics, &PropOpts::with_fixed_step(10.0, RSSStatePV {}));
    prop.until_time_elapsed(prop_time);

    // Check that the STM is correct by back propagating by the previous step, and multiplying by the STM.
    let final_state = prop.dynamics.state.to_cartesian_vec();
    let final_stm = prop.dynamics.stm;
    let final_step = prop.latest_details().step;
    prop.until_time_elapsed(-final_step);

    // And check the difference
    let stm_err = final_stm * prop.dynamics.state.to_cartesian_vec() - final_state;
    let radius_err = stm_err.fixed_rows::<U3>(0).into_owned();
    let velocity_err = stm_err.fixed_rows::<U3>(3).into_owned();

    assert!(radius_err.norm() < 1e-3);
    assert!(velocity_err.norm() < 1e-3);
}
