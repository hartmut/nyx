extern crate nyx_space as nyx;
extern crate pretty_env_logger;

use anise::constants::celestial_objects::{JUPITER_BARYCENTER, MOON, SATURN_BARYCENTER, SUN};
use anise::constants::frames::IAU_EARTH_FRAME;
use nalgebra::Const;
use nyx::cosmic::Orbit;
use nyx::dynamics::orbital::OrbitalDynamics;
use nyx::dynamics::SpacecraftDynamics;
use nyx::io::ExportCfg;
use nyx::md::StateParameter;
use nyx::od::prelude::*;
use nyx::propagators::{IntegratorOptions, Propagator};
use nyx::time::{Epoch, TimeUnits, Unit};
use nyx::utils::rss_orbit_errors;
use nyx::Spacecraft;
use nyx_space::mc::StateDispersion;
use nyx_space::propagators::IntegratorMethod;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

use anise::{constants::frames::EARTH_J2000, prelude::Almanac};
use rstest::*;
use std::sync::Arc;

#[fixture]
fn almanac() -> Arc<Almanac> {
    use crate::test_almanac_arcd;
    test_almanac_arcd()
}

/// Using identically configured ground stations for all tests.
#[fixture]
fn devices(almanac: Arc<Almanac>) -> BTreeMap<String, GroundStation> {
    let iau_earth = almanac.frame_from_uid(IAU_EARTH_FRAME).unwrap();
    let elevation_mask = 10.0;
    let integration_time = None; //Some(60 * Unit::Second);

    let dss65_madrid = GroundStation::dss65_madrid(
        elevation_mask,
        StochasticNoise::default_range_km(),
        StochasticNoise::default_doppler_km_s(),
        iau_earth,
    )
    // .with_msr_type(MeasurementType::Azimuth, StochasticNoise::MIN)
    // .with_msr_type(MeasurementType::Elevation, StochasticNoise::MIN)
    // .without_msr_type(MeasurementType::Range)
    // .without_msr_type(MeasurementType::Doppler)
    .with_integration_time(integration_time);

    let dss34_canberra = GroundStation::dss34_canberra(
        elevation_mask,
        StochasticNoise::default_range_km(),
        StochasticNoise::default_doppler_km_s(),
        iau_earth,
    )
    // .with_msr_type(MeasurementType::Azimuth, StochasticNoise::MIN)
    // .with_msr_type(MeasurementType::Elevation, StochasticNoise::MIN)
    // .without_msr_type(MeasurementType::Range)
    // .without_msr_type(MeasurementType::Doppler)
    .with_integration_time(integration_time);

    let dss13_goldstone = GroundStation::dss13_goldstone(
        elevation_mask,
        StochasticNoise::default_range_km(),
        StochasticNoise::default_doppler_km_s(),
        iau_earth,
    )
    // .with_msr_type(MeasurementType::Azimuth, StochasticNoise::MIN)
    // .with_msr_type(MeasurementType::Elevation, StochasticNoise::MIN)
    // .without_msr_type(MeasurementType::Range)
    // .without_msr_type(MeasurementType::Doppler)
    .with_integration_time(integration_time);

    let mut devices = BTreeMap::new();
    devices.insert(dss65_madrid.name(), dss65_madrid);
    devices.insert(dss34_canberra.name(), dss34_canberra);
    devices.insert(dss13_goldstone.name(), dss13_goldstone);

    devices
}

#[fixture]
fn initial_state(almanac: Arc<Almanac>) -> Spacecraft {
    // Define state information.
    let eme2k = almanac.frame_from_uid(EARTH_J2000).unwrap();
    let dt = Epoch::from_gregorian_utc_at_midnight(2020, 1, 1);
    Spacecraft::from(Orbit::keplerian(
        22000.0, 0.01, 30.0, 80.0, 40.0, 0.0, dt, eme2k,
    ))
}

#[fixture]
fn initial_estimate(initial_state: Spacecraft) -> KfEstimate<Spacecraft> {
    // Disperse the initial state based on some orbital elements errors given from ULA Atlas 5 user guide, table 2.3.3-1 <https://www.ulalaunch.com/docs/default-source/rockets/atlasvusersguide2010a.pdf>
    // This assumes that the errors are ONE TENTH of the values given in the table. It assumes that the launch provider has provided an initial state vector, whose error is lower than the injection errors.
    // The initial covariance is computed based on the realized dispersions.
    let initial_estimate = KfEstimate::disperse_from_diag(
        initial_state,
        vec![
            StateDispersion::zero_mean(StateParameter::Inclination, 0.0025),
            StateDispersion::zero_mean(StateParameter::RAAN, 0.022),
            StateDispersion::zero_mean(StateParameter::AoP, 0.02),
        ],
        Some(0),
    )
    .unwrap();

    println!("Initial estimate:\n{}", initial_estimate);

    initial_estimate
}

#[fixture]
fn truth_setup() -> Propagator<SpacecraftDynamics> {
    let step_size = 60.0 * Unit::Second;
    let opts = IntegratorOptions::with_max_step(step_size);

    let bodies = vec![MOON, SUN, JUPITER_BARYCENTER, SATURN_BARYCENTER];
    let orbital_dyn = OrbitalDynamics::point_masses(bodies);
    Propagator::new(
        SpacecraftDynamics::new(orbital_dyn),
        IntegratorMethod::DormandPrince78,
        opts,
    )
}

#[fixture]
fn estimator_setup() -> Propagator<SpacecraftDynamics> {
    // Now that we have the truth data, let's start an OD with no noise at all and compute the estimates.
    // We expect the estimated orbit to be _nearly_ perfect because we've removed SATURN_BARYCENTER from the estimated trajectory

    let step_size = 60.0 * Unit::Second;
    let opts = IntegratorOptions::with_max_step(step_size);

    let bodies = vec![MOON, SUN, JUPITER_BARYCENTER, SATURN_BARYCENTER];
    let estimator = SpacecraftDynamics::new(OrbitalDynamics::point_masses(bodies));

    Propagator::new(estimator, IntegratorMethod::DormandPrince78, opts)
}

/*
 * These tests check that if we start with a state deviation in the estimate, the filter will eventually converge back.
 * These tests do NOT check that the filter will converge if the initial state in the propagator has that state deviation.
 * The latter would require iteration and smoothing before playing with an EKF. This will be handled in a subsequent version.
**/

#[allow(clippy::identity_op)]
#[rstest]
fn od_robust_test_ekf_rng_dop_az_el(
    almanac: Arc<Almanac>,
    devices: BTreeMap<String, GroundStation>,
    initial_state: Spacecraft,
    initial_estimate: KfEstimate<Spacecraft>,
    truth_setup: Propagator<SpacecraftDynamics>,
    estimator_setup: Propagator<SpacecraftDynamics>,
) {
    let _ = pretty_env_logger::try_init();

    let iau_earth = almanac.frame_from_uid(IAU_EARTH_FRAME).unwrap();
    // Define the ground stations.
    let ekf_num_meas = 10;
    // Set the disable time to be very low to test enable/disable sequence
    let ekf_disable_time = 3 * Unit::Minute;
    let elevation_mask = 0.0;

    let dss65_madrid = GroundStation::dss65_madrid(
        elevation_mask,
        StochasticNoise::default_range_km(),
        StochasticNoise::default_doppler_km_s(),
        iau_earth,
    );
    let dss34_canberra = GroundStation::dss34_canberra(
        elevation_mask,
        StochasticNoise::default_range_km(),
        StochasticNoise::default_doppler_km_s(),
        iau_earth,
    );

    // Define the tracking configurations
    let configs = BTreeMap::from([
        (
            dss65_madrid.name.clone(),
            TrkConfig::from_sample_rate(60.seconds()),
        ),
        (
            dss34_canberra.name.clone(),
            TrkConfig::from_sample_rate(60.seconds()),
        ),
    ]);

    // Define the propagator information.
    let prop_time = 1.1 * initial_state.orbit.period().unwrap();

    let initial_state_dev = initial_estimate.nominal_state;
    let (init_rss_pos_km, init_rss_vel_km_s) =
        rss_orbit_errors(&initial_state.orbit, &initial_state_dev.orbit);

    println!("Truth initial state:\n{initial_state}\n{initial_state:x}");
    println!("Filter initial state:\n{initial_state_dev}\n{initial_state_dev:x}");
    println!(
        "Initial state dev:\t{:.3} m\t{:.3} m/s\n{}",
        init_rss_pos_km * 1e3,
        init_rss_vel_km_s * 1e3,
        (initial_state.orbit - initial_state_dev.orbit).unwrap()
    );

    let (_, traj) = truth_setup
        .with(initial_state, almanac.clone())
        .for_duration_with_traj(prop_time)
        .unwrap();

    // Simulate tracking data
    let mut arc_sim = TrackingArcSim::with_seed(devices.clone(), traj.clone(), configs, 0).unwrap();
    arc_sim.build_schedule(almanac.clone()).unwrap();

    let arc = arc_sim.generate_measurements(almanac.clone()).unwrap();

    // And serialize to disk
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "output_data",
        "ekf_rng_dpl_az_el_arc.parquet",
    ]
    .iter()
    .collect();

    arc.to_parquet_simple(&path).unwrap();

    println!("{arc}\n{arc:?}");
    // Reload
    let reloaded = TrackingDataArc::from_parquet(&path).unwrap();
    assert_eq!(reloaded, arc);

    let prop_est = estimator_setup.with(initial_state_dev.with_stm(), almanac.clone());

    // Define the process noise to assume an unmodeled acceleration on X, Y and Z in the EME2000 frame
    let sigma_q = 5e-10_f64.powi(2);
    let process_noise = SNC3::from_diagonal(2 * Unit::Minute, &[sigma_q, sigma_q, sigma_q]);

    let kf = KF::new(initial_estimate, process_noise);

    let trig = EkfTrigger::new(ekf_num_meas, ekf_disable_time);

    let mut odp = ODProcess::<
        SpacecraftDynamics,
        Const<2>,
        Const<3>,
        KF<Spacecraft, Const<3>, Const<2>>,
        GroundStation,
    >::ekf(
        prop_est,
        kf,
        devices,
        trig,
        Some(ResidRejectCrit::default()),
        almanac,
    );

    // let mut odp = SpacecraftODProcess::ekf(
    //     prop_est, kf, devices, trig, None, // Some(ResidRejectCrit::default()),
    //     almanac,
    // );

    // Let's filter and iterate on the initial subset of the arc to refine the initial estimate
    // let subset = arc.clone().filter_by_offset(..3.hours());
    // let remaining = arc.filter_by_offset(3.hours()..);

    odp.process_arc(&arc).unwrap();
    odp.iterate_arc(&arc, IterationConf::once()).unwrap();

    // Grab the comparison state, which differs from the initial state because of the integration time of the measurements.
    let cmp_state = traj
        .at(odp.estimates[0].state().orbit.epoch)
        .expect("could not find comparison epoch in trajectory");

    let (sm_rss_pos_km, sm_rss_vel_km_s) =
        rss_orbit_errors(&cmp_state.orbit, &odp.estimates[0].state().orbit);

    println!(
        "Initial state error after smoothing:\t{:.3} m\t{:.3} m/s\n{}",
        sm_rss_pos_km * 1e3,
        sm_rss_vel_km_s * 1e3,
        (cmp_state.orbit - odp.estimates[0].state().orbit).unwrap()
    );

    // odp.process_arc(&remaining).unwrap();

    odp.to_parquet(
        &arc,
        path.with_file_name("ekf_rng_dpl_az_el_odp.parquet"),
        ExportCfg::default(),
    )
    .unwrap();

    // Check that the covariance deflated
    let est = &odp.estimates[odp.estimates.len() - 1];
    let final_truth_state = traj.at(est.epoch()).unwrap();

    println!("Estimate:\n{}", est);
    println!("Truth:\n{}", final_truth_state);
    println!(
        "Delta state with truth (epoch match: {}):\n{}",
        final_truth_state.epoch() == est.epoch(),
        (final_truth_state.orbit - est.state().orbit).unwrap()
    );

    for i in 0..6 {
        if est.covar[(i, i)] < 0.0 {
            println!(
                "covar diagonal element negative @ [{}, {}] = {:.3e}-- issue #164",
                i,
                i,
                est.covar[(i, i)],
            );
        }
    }

    assert_eq!(
        final_truth_state.epoch(),
        est.epoch(),
        "time of final EST and TRUTH epochs differ"
    );
    let delta = (est.state().orbit - final_truth_state.orbit).unwrap();
    println!(
        "RMAG error = {:.6} m\tVMAG error = {:.6} m/s",
        delta.rmag_km() * 1e3,
        delta.vmag_km_s() * 1e3
    );

    assert!(
        delta.rmag_km() < 0.06,
        "Position error should be less than 50 meters"
    );
    assert!(
        delta.vmag_km_s() < 2e-4,
        "Velocity error should be on centimeter level"
    );
}
