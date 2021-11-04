extern crate nyx_space as nyx;

use nyx::md::ui::*;
use nyx::opti::multishoot::*;

#[test]
fn orbit_raising() {
    let cosm = Cosm::de438_gmat();
    let eme2k = cosm.frame("EME2000");
    let iau_earth = cosm.frame("IAU_Earth");

    /* Define the parking orbit */
    let epoch = Epoch::from_gregorian_utc_at_noon(2022, 03, 04);
    let start = Orbit::keplerian_altitude(300.0, 0.01, 30.0, 90.0, 90.0, 60.0, epoch, eme2k);

    /* Build the spacecraft -- really only the mass is needed here */
    let sc = Spacecraft {
        orbit: start,
        ..Default::default()
    };

    /* Define the target orbit */
    let target = Orbit::keplerian_altitude(
        1500.0,
        0.01,
        30.0,
        90.0,
        90.0,
        60.0,
        epoch + 30 * TimeUnit::Minute,
        eme2k,
    );

    /* Define the multiple shooting parameters */
    let node_count = 30; // We're targeting 30 minutes in the future, so using 30 nodes

    let prop = Propagator::default(SpacecraftDynamics::new(OrbitalDynamics::two_body()));
    let mut opti = MultipleShooting::linear_altitude_heuristic(
        sc,
        target,
        node_count,
        iau_earth,
        &prop,
        cosm.clone(),
    )
    .unwrap();

    // Check that all nodes are above the surface
    println!("Initial nodes\nNode no,X (km),Y (km),Z (km),Epoch:GregorianUtc");
    for (i, node) in opti.nodes.iter().enumerate() {
        println!(
            "{}, {}, {}, {}, '{}'",
            i,
            node.x,
            node.y,
            node.z,
            node.epoch.as_gregorian_utc_str()
        );
    }

    let multishoot_sol = opti.solve(CostFunction::MinimumFuel).unwrap();

    println!("Final nodes\nNode no,X (km),Y (km),Z (km),Epoch:GregorianUtc");
    for (i, node) in opti.nodes.iter().enumerate() {
        println!(
            "{}, {}, {}, {}, '{}'",
            i,
            node.x,
            node.y,
            node.z,
            node.epoch.as_gregorian_utc_str()
        );
    }

    for (i, traj) in multishoot_sol
        .build_trajectories(&prop)
        .unwrap()
        .iter()
        .enumerate()
    {
        traj.to_csv_with_step(
            &format!("multishoot_to_node_{}.csv", i),
            2 * TimeUnit::Second,
            cosm.clone(),
        )
        .unwrap();
    }

    let solution = &multishoot_sol.solutions[node_count - 1];
    let sc_sol = solution.achieved;

    println!("{}", multishoot_sol);

    // Compute the total delta-v of the solution
    let mut dv_ms = 0.0;
    for sol in &multishoot_sol.solutions {
        dv_ms += sol.correction.norm() * 1e3;
    }
    println!(
        "Multiple shooting solution requires a total of {} m/s",
        dv_ms
    );

    // Just propagate this spacecraft for one orbit for plotting
    let (_, end_traj) = prop
        .with(sc_sol)
        .for_duration_with_traj(2 * TimeUnit::Hour)
        .unwrap();

    end_traj
        .to_csv_with_step(
            &"multishoot_to_end.csv".to_string(),
            2 * TimeUnit::Second,
            cosm.clone(),
        )
        .unwrap();
}
