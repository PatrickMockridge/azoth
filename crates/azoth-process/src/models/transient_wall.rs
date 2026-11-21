//! The explicit finite-difference wall of NeqSim's `TransientWallHeatTransfer`.
//!
//! The single-layer constructor, the interior harmonic-mean conductivity, the Robin
//! boundaries with the outer additional flux, the CFL sub-step split and the unweighted
//! node mean are the class's; `process.trapped_liquid_fire_rupture` is their only caller.

use azoth_core::{AzothError, Result};

/// One wall whose temperature profile advances by explicit finite differences.
#[derive(Debug, Clone)]
pub(crate) struct TransientWall {
    temperature: Vec<f64>,
    conductivity: Vec<f64>,
    density: Vec<f64>,
    heat_capacity: Vec<f64>,
    diffusivity: Vec<f64>,
    num_nodes: usize,
    dx: f64,
    total_thickness: f64,
}

impl TransientWall {
    /// The class's single-layer constructor.
    pub(crate) fn new(
        thickness: f64,
        conductivity: f64,
        density: f64,
        heat_capacity: f64,
        initial_temperature: f64,
        num_nodes: usize,
    ) -> Result<Self> {
        if !(thickness > 0.0
            && thickness.is_finite()
            && conductivity > 0.0
            && conductivity.is_finite()
            && density > 0.0
            && density.is_finite()
            && heat_capacity > 0.0
            && heat_capacity.is_finite())
        {
            return Err(AzothError::invalid_input(
                "wall",
                "the class's constructor refuses a non-positive wall property",
            ));
        }
        if num_nodes < 3 {
            return Err(AzothError::invalid_input(
                "wall_nodes",
                "the class's constructor requires at least three nodes",
            ));
        }
        if !(initial_temperature > 0.0 && initial_temperature.is_finite()) {
            return Err(AzothError::invalid_input(
                "initial_temperature",
                "the class's constructor requires a positive initial temperature",
            ));
        }
        let dx = thickness / (num_nodes as f64 - 1.0);
        let alpha = conductivity / (density * heat_capacity);
        if !(alpha > 0.0 && alpha.is_finite()) {
            return Err(AzothError::invalid_input(
                "wall",
                "the wall's thermal diffusivity is not positive and finite",
            ));
        }
        Ok(Self {
            temperature: vec![initial_temperature; num_nodes],
            conductivity: vec![conductivity; num_nodes],
            density: vec![density; num_nodes],
            heat_capacity: vec![heat_capacity; num_nodes],
            diffusivity: vec![alpha; num_nodes],
            num_nodes,
            dx,
            total_thickness: thickness,
        })
    }

    /// Advance the profile by one step, subdividing it where the explicit scheme would be
    /// unstable.
    pub(crate) fn advance_time_step(
        &mut self,
        dt: f64,
        inner_fluid_temperature: f64,
        inner_htc: f64,
        outer_ambient_temperature: f64,
        outer_htc: f64,
        additional_outer_flux: f64,
    ) -> Result<()> {
        if !(dt > 0.0 && dt.is_finite()) {
            return Err(AzothError::invalid_input(
                "time_step",
                "the class refuses a non-positive time step",
            ));
        }
        let max_alpha = self
            .diffusivity
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let max_dt = 0.5 * self.dx * self.dx / max_alpha;
        if !(max_dt > 0.0 && max_dt.is_finite()) {
            return Err(AzothError::invalid_input(
                "wall",
                "the CFL step of the wall is not positive and finite",
            ));
        }
        if dt > max_dt {
            let sub_steps = (dt / max_dt).ceil() as usize + 1;
            let sub_dt = dt / sub_steps as f64;
            for _ in 0..sub_steps {
                self.advance_time_step_internal(
                    sub_dt,
                    inner_fluid_temperature,
                    inner_htc,
                    outer_ambient_temperature,
                    outer_htc,
                    additional_outer_flux,
                );
            }
        } else {
            self.advance_time_step_internal(
                dt,
                inner_fluid_temperature,
                inner_htc,
                outer_ambient_temperature,
                outer_htc,
                additional_outer_flux,
            );
        }
        Ok(())
    }

    fn advance_time_step_internal(
        &mut self,
        dt: f64,
        inner_fluid_temperature: f64,
        inner_htc: f64,
        outer_ambient_temperature: f64,
        outer_htc: f64,
        additional_outer_flux: f64,
    ) {
        let mut next = vec![0.0; self.num_nodes];
        for (i, slot) in next.iter_mut().enumerate().take(self.num_nodes - 1).skip(1) {
            let k_left = 2.0 * self.conductivity[i] * self.conductivity[i - 1]
                / (self.conductivity[i] + self.conductivity[i - 1]);
            let k_right = 2.0 * self.conductivity[i] * self.conductivity[i + 1]
                / (self.conductivity[i] + self.conductivity[i + 1]);
            let k_effective = (k_left + k_right) / 2.0;
            let alpha_effective = k_effective / (self.density[i] * self.heat_capacity[i]);
            let fourier = alpha_effective * dt / (self.dx * self.dx);
            *slot = self.temperature[i]
                + fourier
                    * (self.temperature[i - 1] - 2.0 * self.temperature[i]
                        + self.temperature[i + 1]);
        }

        let k_inner = self.conductivity[0];
        let biot_inner = inner_htc * self.dx / k_inner;
        let fourier_inner = self.diffusivity[0] * dt / (self.dx * self.dx);
        next[0] = self.temperature[0]
            + 2.0
                * fourier_inner
                * (self.temperature[1] - self.temperature[0]
                    + biot_inner * (inner_fluid_temperature - self.temperature[0]));

        let n = self.num_nodes - 1;
        let k_outer = self.conductivity[n];
        let biot_outer = outer_htc * self.dx / k_outer;
        let fourier_outer = self.diffusivity[n] * dt / (self.dx * self.dx);
        let flux_term = additional_outer_flux * self.dx / k_outer;
        next[n] = self.temperature[n]
            + 2.0
                * fourier_outer
                * (self.temperature[n - 1] - self.temperature[n]
                    + biot_outer * (outer_ambient_temperature - self.temperature[n])
                    + flux_term);

        self.temperature = next;
    }

    /// The inner surface temperature, K.
    pub(crate) fn inner_wall_temperature(&self) -> f64 {
        self.temperature[0]
    }

    /// The outer surface temperature, K.
    pub(crate) fn outer_wall_temperature(&self) -> f64 {
        self.temperature[self.num_nodes - 1]
    }

    /// The unweighted node mean, K.
    pub(crate) fn mean_wall_temperature(&self) -> f64 {
        self.temperature.iter().sum::<f64>() / self.num_nodes as f64
    }

    /// The heat absorbed over a step, J.
    pub(crate) fn heat_absorbed(&self, wall_area: f64, previous_mean_temperature: f64) -> f64 {
        let delta = self.mean_wall_temperature() - previous_mean_temperature;
        let per_node = self
            .density
            .iter()
            .zip(&self.heat_capacity)
            .map(|(density, capacity)| density * capacity)
            .sum::<f64>()
            / self.num_nodes as f64;
        per_node * self.total_thickness * wall_area * delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wall_with_no_drive_stays_uniform_through_the_cfl_substeps() {
        let mut wall =
            TransientWall::new(0.01, 45.0, 7850.0, 500.0, 300.0, 15).expect("the wall builds");
        wall.advance_time_step(2.0, 300.0, 500.0, 300.0, 35.0, 0.0)
            .expect("the step advances");
        assert!((wall.inner_wall_temperature() - 300.0).abs() < 1.0e-12);
        assert!((wall.outer_wall_temperature() - 300.0).abs() < 1.0e-12);
        assert!(wall.heat_absorbed(1.0, 300.0).abs() < 1.0e-9);
    }
}
