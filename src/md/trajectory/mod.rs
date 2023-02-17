/*
    Nyx, blazing fast astrodynamics
    Copyright (C) 2023 Christopher Rabotin <christopher.rabotin@gmail.com>

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

pub mod btraj;
pub(crate) mod spline;
mod traj;
mod traj_it;
// pub(crate) use traj_it::TrajIterator;

pub(crate) use traj::interpolate;
pub use traj::Traj;

use self::spline::INTERPOLATION_SAMPLES;

use super::StateParameter;
use crate::linalg::allocator::Allocator;
use crate::linalg::DefaultAllocator;
use crate::polyfit::hermite::hermite_eval;
use crate::time::{Duration, Epoch};
use crate::{NyxError, Orbit, Spacecraft, State};

use std::error::Error;
use std::fmt;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TrajError {
    EventNotFound {
        start: Epoch,
        end: Epoch,
        event: String,
    },
    NoInterpolationData(Epoch),
    CreationError(String),
    OutOfSpline {
        req_epoch: Epoch,
        req_dur: Duration,
        spline_dur: Duration,
    },
}

impl fmt::Display for TrajError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::EventNotFound { start, end, event } => {
                write!(f, "Event {event} not found between {start} and {end}")
            }
            Self::CreationError(reason) => write!(f, "Failed to create trajectory: {reason}"),
            Self::NoInterpolationData(e) => write!(f, "No interpolation data at {e}"),
            Self::OutOfSpline {
                req_epoch,
                req_dur,
                spline_dur,
            } => {
                write!(f, "Probable bug: Requested epoch {req_epoch}, corresponding to an offset of {req_dur} in a spline of duration {spline_dur}")
            }
        }
    }
}

impl Error for TrajError {}

pub trait InterpState: State
where
    Self: Sized,
    DefaultAllocator: Allocator<f64, Self::Size>
        + Allocator<f64, Self::Size, Self::Size>
        + Allocator<f64, Self::VecLength>,
{
    /// Return the parameters in order
    fn params() -> Vec<StateParameter>;

    /// Return the requested parameter and its time derivative
    fn value_and_deriv(&self, param: &StateParameter) -> Result<(f64, f64), NyxError> {
        Ok((self.value(param)?, self.deriv(param)?))
    }

    /// Return the time derivative requested parameter
    fn deriv(&self, param: &StateParameter) -> Result<f64, NyxError> {
        Ok(self.value_and_deriv(param)?.1)
    }

    /// Sets the requested parameter
    fn set_value_and_deriv(
        &mut self,
        param: &StateParameter,
        value: f64,
        value_dt: f64,
    ) -> Result<(), NyxError>;

    /// Interpolates a new state at the provided epochs given a slice of states.
    fn interpolate(self, epoch: Epoch, states: &[Self]) -> Result<Self, NyxError>;
}

impl InterpState for Orbit {
    fn params() -> Vec<StateParameter> {
        vec![
            StateParameter::X,
            StateParameter::Y,
            StateParameter::Z,
            StateParameter::VX,
            StateParameter::VY,
            StateParameter::VZ,
        ]
    }

    fn interpolate(self, epoch: Epoch, states: &[Self]) -> Result<Self, NyxError> {
        // This is copied from ANISE

        // Statically allocated arrays of the maximum number of samples
        let mut epochs_tdb = [0.0; INTERPOLATION_SAMPLES];
        let mut xs = [0.0; INTERPOLATION_SAMPLES];
        let mut ys = [0.0; INTERPOLATION_SAMPLES];
        let mut zs = [0.0; INTERPOLATION_SAMPLES];
        let mut vxs = [0.0; INTERPOLATION_SAMPLES];
        let mut vys = [0.0; INTERPOLATION_SAMPLES];
        let mut vzs = [0.0; INTERPOLATION_SAMPLES];

        for (cno, state) in states.iter().enumerate() {
            xs[cno] = state.x;
            ys[cno] = state.y;
            zs[cno] = state.z;
            vxs[cno] = state.vx;
            vys[cno] = state.vy;
            vzs[cno] = state.vz;
            epochs_tdb[cno] = state.epoch().to_tdb_seconds();
        }

        // TODO: Once I switch to using ANISE, this should use the same function as ANISE and not a clone.
        let (x_km, vx_km_s) = hermite_eval(
            &epochs_tdb[..states.len()],
            &xs[..states.len()],
            &vxs[..states.len()],
            epoch.to_et_seconds(),
        )?;

        let (y_km, vy_km_s) = hermite_eval(
            &epochs_tdb[..states.len()],
            &ys[..states.len()],
            &vys[..states.len()],
            epoch.to_et_seconds(),
        )?;

        let (z_km, vz_km_s) = hermite_eval(
            &epochs_tdb[..states.len()],
            &zs[..states.len()],
            &vzs[..states.len()],
            epoch.to_et_seconds(),
        )?;

        // And build the result
        let mut me = self;
        me.x = x_km;
        me.y = y_km;
        me.z = z_km;
        me.vx = vx_km_s;
        me.vy = vy_km_s;
        me.vz = vz_km_s;

        Ok(me)
    }

    fn value_and_deriv(&self, param: &StateParameter) -> Result<(f64, f64), NyxError> {
        match *param {
            StateParameter::X => Ok((self.x, self.vx)),
            StateParameter::Y => Ok((self.y, self.vy)),
            StateParameter::Z => Ok((self.z, self.vz)),
            StateParameter::VX => Ok((self.vx, 0.0)),
            StateParameter::VY => Ok((self.vy, 0.0)),
            StateParameter::VZ => Ok((self.vz, 0.0)),
            _ => Err(NyxError::StateParameterUnavailable),
        }
    }

    fn set_value_and_deriv(
        &mut self,
        param: &StateParameter,
        value: f64,
        _: f64,
    ) -> Result<(), NyxError> {
        match *param {
            StateParameter::X => {
                self.x = value;
            }
            StateParameter::Y => {
                self.y = value;
            }
            StateParameter::Z => {
                self.z = value;
            }
            StateParameter::VX => {
                self.vx = value;
            }
            StateParameter::VY => {
                self.vy = value;
            }
            StateParameter::VZ => {
                self.vz = value;
            }

            _ => return Err(NyxError::StateParameterUnavailable),
        }
        Ok(())
    }
}

impl InterpState for Spacecraft {
    fn params() -> Vec<StateParameter> {
        vec![
            StateParameter::X,
            StateParameter::Y,
            StateParameter::Z,
            StateParameter::VX,
            StateParameter::VY,
            StateParameter::VZ,
            StateParameter::FuelMass,
        ]
    }

    fn interpolate(self, epoch: Epoch, states: &[Self]) -> Result<Self, NyxError> {
        // Use the Orbit interpolation first.
        let orbit = Orbit::interpolate(
            self.orbit,
            epoch,
            &states.iter().map(|sc| sc.orbit).collect::<Vec<_>>(),
        )?;

        // Fuel is linearly interpolated -- should really be a Lagrange interpolation here
        let fuel_kg = (states.last().unwrap().fuel_mass_kg - states.first().unwrap().fuel_mass_kg)
            / (states.last().unwrap().epoch().to_tdb_seconds()
                - states.first().unwrap().epoch().to_tdb_seconds());

        let mut me = self.with_orbit(orbit);
        me.fuel_mass_kg = fuel_kg;

        Ok(me)
    }

    fn value_and_deriv(&self, param: &StateParameter) -> Result<(f64, f64), NyxError> {
        match *param {
            StateParameter::X => Ok((self.orbit.x, self.orbit.vx)),
            StateParameter::Y => Ok((self.orbit.y, self.orbit.vy)),
            StateParameter::Z => Ok((self.orbit.z, self.orbit.vz)),
            StateParameter::VX => Ok((self.orbit.vx, 0.0)),
            StateParameter::VY => Ok((self.orbit.vy, 0.0)),
            StateParameter::VZ => Ok((self.orbit.vz, 0.0)),
            StateParameter::FuelMass => Ok((self.fuel_mass_kg, 0.0)),
            _ => Err(NyxError::StateParameterUnavailable),
        }
    }

    fn set_value_and_deriv(
        &mut self,
        param: &StateParameter,
        value: f64,
        _: f64,
    ) -> Result<(), NyxError> {
        match *param {
            StateParameter::X => {
                self.orbit.x = value;
            }
            StateParameter::Y => {
                self.orbit.y = value;
            }
            StateParameter::Z => {
                self.orbit.z = value;
            }
            StateParameter::VX => {
                self.orbit.vx = value;
            }
            StateParameter::VY => {
                self.orbit.vy = value;
            }
            StateParameter::VZ => {
                self.orbit.vz = value;
            }
            StateParameter::Cr => self.cr = value,
            StateParameter::Cd => self.cd = value,
            StateParameter::FuelMass => self.fuel_mass_kg = value,
            _ => return Err(NyxError::StateParameterUnavailable),
        }
        Ok(())
    }
}
