use nyx_space::io::ConfigRepr;
use nyx_space::md::prelude::*;
use nyx_space::md::trajectory::ExportCfg;
use nyx_space::od::prelude::*;
use nyx_space::od::simulator::TrackingArcSim;
use nyx_space::od::simulator::TrkConfig;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use anise::{constants::frames::EARTH_J2000, prelude::Almanac};
use rstest::*;
use std::sync::Arc;

#[fixture]
fn almanac() -> Arc<Almanac> {
    use crate::test_almanac_arcd;
    test_almanac_arcd()
}

// Consider changing this to a fixture to run the moduli tests.
#[rstest]
fn continuous_tracking_cov_test(almanac: Arc<Almanac>) {
    // Test that continuous tracking
    let _ = pretty_env_logger::try_init();

    // Dummy state
    let orbit = Orbit::try_keplerian_altitude(
        150_000.0,
        1e-2,
        30.0,
        45.0,
        75.0,
        23.4,
        Epoch::from_str("2023-02-22T19:18:17.16 UTC").unwrap(),
        almanac.frame_from_uid(EARTH_J2000).unwrap(),
    )
    .unwrap();

    // Generate a trajectory
    let (_, trajectory) = Propagator::default(SpacecraftDynamics::new(OrbitalDynamics::two_body()))
        .with(orbit.into(), almanac.clone())
        .for_duration_with_traj(0.25 * orbit.period().unwrap())
        .unwrap();

    println!("{trajectory}");

    // Save the trajectory to parquet
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "output_data",
        "tracking_truth_ephem.parquet",
    ]
    .iter()
    .collect();

    trajectory
        .to_parquet_with_cfg(
            path,
            ExportCfg {
                timestamp: true,
                ..Default::default()
            },
            almanac.clone(),
        )
        .unwrap();

    // Load the ground stations from the test data.
    let ground_station_file: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "data",
        "tests",
        "config",
        "many_ground_stations.yaml",
    ]
    .iter()
    .collect();

    let mut devices = BTreeMap::new();
    for gs in GroundStation::load_many(ground_station_file).unwrap() {
        devices.insert(
            gs.name.clone(),
            gs.with_msr_type(
                MeasurementType::Azimuth,
                StochasticNoise::default_angle_deg(),
            )
            .with_msr_type(
                MeasurementType::Elevation,
                StochasticNoise::default_angle_deg(),
            ),
        );
    }

    // Load the tracking configuration from the test data.
    let trkconfg_yaml: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "data",
        "tests",
        "config",
        "tracking_cfg.yaml",
    ]
    .iter()
    .collect();

    let configs: BTreeMap<String, TrkConfig> = TrkConfig::load_named(trkconfg_yaml).unwrap();

    dbg!(&configs);

    // Build the tracking arc simulation to generate a "standard measurement".
    let mut trk =
        TrackingArcSim::<Spacecraft, GroundStation>::with_seed(devices, trajectory, configs, 12345)
            .unwrap();

    trk.build_schedule(almanac.clone()).unwrap();
    let mut arc = trk.generate_measurements(almanac).unwrap();

    // Assume JPL DSN Code is used, cf. DSN docs 214, section 2.2.2.2.
    let jpl_dsn_code_length_km = 75660.0;
    arc.set_moduli(MeasurementType::Range, jpl_dsn_code_length_km);
    arc.apply_moduli();

    // And serialize to disk
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "output_data",
        "simple_arc.parquet",
    ]
    .iter()
    .collect();

    let output_fn = arc.to_parquet_simple(path).unwrap();
    println!("[{}] {arc}", output_fn.to_string_lossy());

    // Now read this file back in.
    let arc_rtn = TrackingDataArc::from_parquet(output_fn).unwrap();

    println!("{arc_rtn}");

    assert_eq!(arc.measurements.len(), 96734);
    // Check that we've loaded all of the measurements
    assert_eq!(arc_rtn.measurements.len(), arc.measurements.len());
    assert_eq!(arc_rtn.unique(), arc.unique());

    // Serialize as TDM
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "output_data", "simple_arc.tdm"]
        .iter()
        .collect();

    let mut aliases = HashMap::new();
    aliases.insert("Demo Ground Station".to_string(), "Fake GS".to_string());

    let output_fn = arc
        .clone()
        .to_tdm_file(
            "MySpacecraft".to_string(),
            Some(aliases.clone()),
            path,
            ExportCfg::default(),
        )
        .unwrap();

    // Read back from TDM
    let arc_tdm = TrackingDataArc::from_tdm(output_fn, None).unwrap();
    println!("{arc_tdm}");

    // Check everything but the source, since it'll be set when read from TDM.
    assert_eq!(arc_tdm.len(), arc.len());
    assert_eq!(arc_tdm.start_epoch(), arc.start_epoch());
    assert_eq!(arc_tdm.end_epoch(), arc.end_epoch());
    assert_eq!(arc_tdm.unique(), arc.unique());
    // Check that we have multiplied back and divided back correctly.
    assert_eq!(arc_tdm.measurements, arc.measurements);

    // Test the downsampling
    let tdm_failed_downsample = arc_tdm.clone().downsample(0.1.seconds());
    assert_eq!(
        tdm_failed_downsample.len(),
        arc_tdm.len(),
        "downsampling should have failed because it's upsampling"
    );

    let arc_downsample = arc_tdm.clone().downsample(10.seconds());
    println!("{arc_downsample}");
    assert_eq!(
        arc_downsample.len(),
        arc_tdm.len() / 10,
        "downsampling has wrong sample count"
    );

    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "output_data",
        "simple_arc_downsampled.parquet",
    ]
    .iter()
    .collect();

    arc_downsample
        .to_parquet(path, ExportCfg::default())
        .unwrap();
}
