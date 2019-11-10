use super::celestial::CelestialDynamics;
use super::na::{Vector1, VectorN, U6, U7};
use super::propulsion::Propulsion;
use super::solarpressure::SolarPressure;
use super::thrustctrl::ThrustControl;
use super::Dynamics;
use celestia::{Geoid, State};
use std::fmt;
use std::marker::PhantomData;

pub struct Spacecraft<'a, T: ThrustControl> {
    pub celestial: &'a mut CelestialDynamics<'a>,
    pub prop: Option<&'a mut Propulsion<'a, T>>,
    pub srp: Option<&'a mut SolarPressure<'a>>,
    /// in kg
    pub dry_mass: f64,
    _marker: PhantomData<T>,
}

impl<'a, T: ThrustControl> Spacecraft<'a, T> {
    /// Initialize a Spacecraft with a set of celestial dynamics and a propulsion subsystem.
    pub fn with_prop(
        celestial: &'a mut CelestialDynamics<'a>,
        prop: &'a mut Propulsion<'a, T>,
        dry_mass: f64,
    ) -> Self {
        // Set the dry mass of the propulsion system
        prop.dry_mass = dry_mass;
        Self {
            celestial,
            prop: Some(prop),
            srp: None,
            dry_mass,
            _marker: PhantomData,
        }
    }

    /// Initialize a Spacecraft with a set of celestial dynamics and with SRP enabled.
    pub fn with_srp(
        celestial: &'a mut CelestialDynamics<'a>,
        srp: &'a mut SolarPressure<'a>,
        dry_mass: f64,
    ) -> Self {
        // Set the dry mass of the propulsion system
        Self {
            celestial,
            prop: None,
            srp: Some(srp),
            dry_mass,
            _marker: PhantomData,
        }
    }
}

impl<'a, T: ThrustControl> Dynamics for Spacecraft<'a, T> {
    type StateSize = U7;
    type StateType = SpacecraftState;

    fn time(&self) -> f64 {
        self.celestial.time()
    }

    fn state(&self) -> Self::StateType {
        SpacecraftState {
            orbit: self.celestial.state(),
            dry_mass: if let Some(prop) = &self.prop {
                prop.dry_mass
            } else {
                0.0
            },
            fuel_mass: if let Some(prop) = &self.prop {
                prop.state()
            } else {
                0.0
            },
        }
    }

    fn state_vector(&self) -> VectorN<f64, Self::StateSize> {
        let fuel_mass = if let Some(prop) = &self.prop {
            prop.fuel_mass
        } else {
            0.0
        };
        VectorN::<f64, U7>::from_iterator(
            self.celestial
                .state_vector()
                .iter()
                .chain(Vector1::new(fuel_mass).iter())
                .cloned(),
        )
    }

    fn set_state(&mut self, new_t: f64, new_state: &VectorN<f64, Self::StateSize>) {
        let celestial_state = new_state.fixed_rows::<U6>(0).into_owned();
        self.celestial.set_state(new_t, &celestial_state);
        if let Some(prop) = self.prop.as_mut() {
            prop.set_state(new_t, new_state);
        }
    }

    fn eom(&self, t: f64, state: &VectorN<f64, Self::StateSize>) -> VectorN<f64, Self::StateSize> {
        // Compute the celestial dynamics
        let celestial_state = state.fixed_rows::<U6>(0).into_owned();
        let d_x_celestial = self.celestial.eom(t, &celestial_state);
        let mut d_x = VectorN::<f64, U7>::from_iterator(
            d_x_celestial
                .iter()
                .chain(Vector1::new(0.0).iter())
                .cloned(),
        );
        let mut total_mass = self.dry_mass;
        // Now compute the other dynamics as needed.
        if let Some(prop) = &self.prop {
            let prop_dt = prop.eom(t, state);
            // Add the fuel mass to the total mass, minus the change in fuel
            total_mass += prop.fuel_mass + prop_dt[6];
            d_x += prop_dt;
        }
        // Now compute the SRP if applicable
        if let Some(srp) = &self.srp {
            // Hide the total spacecraft mass in the state.
            let mut srp_state = state.to_owned();
            srp_state[6] = total_mass;
            d_x += srp.eom(t, &srp_state);
        }
        d_x
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SpacecraftState {
    pub orbit: State<Geoid>,
    pub dry_mass: f64,
    pub fuel_mass: f64,
}

impl fmt::Display for SpacecraftState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:o}\t{} kg", self.orbit, self.dry_mass + self.fuel_mass)
    }
}