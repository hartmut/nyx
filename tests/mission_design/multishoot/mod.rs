extern crate nyx_space as nyx;

use nyx::dynamics::guidance::Thruster;
use nyx::md::ui::*;
use nyx::opti::multishoot::*;
use std::str::FromStr;

#[test]
fn landing_demo() {
    if pretty_env_logger::try_init().is_err() {
        println!("could not init env_logger");
    }

    const SITE_LAT_DEG: f64 = -86.798;
    const SITE_LONG_DEG: f64 = -21.150;
    const SITE_HEIGHT_KM: f64 = 0.4;
    const ALTITUDE_BUFFER_KM: f64 = 0.2; // Add 200 meters of buffer
    let cosm = Cosm::de438_gmat();
    let moonj2k = cosm.frame("Luna");

    /* *** */
    /* Define the landing epoch, landing site, and initial orbit. */
    /* *** */
    // Landing epoch
    let e_landing = Epoch::from_str("2023-11-25T14:20:00.0").unwrap();
    let vertical_landing_duration = 2 * TimeUnit::Minute;
    let e_pre_landing = e_landing - vertical_landing_duration;
    // Landing site
    let ls = Orbit::from_geodesic(
        SITE_LAT_DEG,
        SITE_LONG_DEG,
        SITE_HEIGHT_KM + ALTITUDE_BUFFER_KM,
        e_pre_landing,
        cosm.frame("IAU Moon"),
    );

    // And of DRM
    let start_orbit = Orbit::cartesian(
        90.17852649,
        -36.46422273,
        -1757.51628437,
        -1.50058776,
        -0.82699041,
        -0.10562691,
        e_landing,
        moonj2k,
    );

    let thruster = Thruster {
        isp: 300.0,
        thrust: 6.0 * 667.233,
    };
    // Masses from δCDR
    let dry_mass_kg = 1100.0;
    let fuel_mass_kg = 1479.4 - 44.353;
    let xl1 = Spacecraft::from_thruster(
        start_orbit,
        dry_mass_kg,
        fuel_mass_kg,
        thruster,
        GuidanceMode::Coast,
    );

    /* *** */
    /* Run the differential corrector for the initial guess of the velocity vector. */
    /* *** */
    // Convert the landing site into the same frame as the spacecraft and use that as targeting values
    let ls_luna = cosm.frame_chg(&ls, moonj2k);

    let prop = Propagator::default(SpacecraftDynamics::new(OrbitalDynamics::two_body()));

    let pdi_start = prop.with(xl1).for_duration(-7 * TimeUnit::Minute).unwrap();

    println!("Start: {}", pdi_start);
    println!(
        "Start: |r| = {:.4} km\t|v| = {:.4} km/s",
        pdi_start.orbit.rmag(),
        pdi_start.orbit.vmag()
    );

    println!("LANDING SITE: {}", ls_luna);
    println!(
        "LANDING SITE slant angle: φ = {} deg",
        pdi_start
            .orbit
            .r_hat()
            .dot(&ls_luna.r_hat())
            .acos()
            .to_degrees()
    );

    // And run the multiple shooting algorithm
    let mut opti = MultipleShooting::equidistant_nodes(pdi_start, ls_luna, 7 * 3, &prop).unwrap();
    let sc_near_ls = opti.solve(CostFunction::MinimumFuel).unwrap();
    // Now, try to land with zero velocity and check again the accuracy
    let objectives = vec![Objective::new(StateParameter::Vmag, 1.0e-3)];
    let tgt = Targeter::delta_v(&prop, objectives);
    let sc_landing_tgt = tgt
        .try_achieve_from(sc_near_ls, sc_near_ls.epoch(), e_landing)
        .unwrap();
    // Check r_miss and v_miss
    let r_miss = (sc_landing_tgt.achieved.orbit.radius() - ls_luna.radius()).norm();
    let v_miss = (sc_landing_tgt.achieved.orbit.velocity() - ls_luna.velocity()).norm();
    println!(
        "\nFINALLY\n{}\n\tr_miss = {:.1} m\tv_miss = {:.1} m/s",
        sc_landing_tgt,
        r_miss * 1e3,
        v_miss * 1e3
    );
}
