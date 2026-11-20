"""The explicit finite-difference wall of NeqSim's ``TransientWallHeatTransfer``.

The Python twin of ``crates/azoth-process/src/models/transient_wall.rs``: the single-layer
constructor, the interior harmonic-mean conductivity, the Robin boundaries with the outer
additional flux, the CFL sub-step split and the unweighted node mean.
"""

from __future__ import annotations

import math

from azoth.core.errors import InvalidInputError


class TransientWall:
    """One wall whose temperature profile advances by explicit finite differences."""

    def __init__(
        self,
        thickness: float,
        conductivity: float,
        density: float,
        heat_capacity: float,
        initial_temperature: float,
        num_nodes: int,
    ) -> None:
        if not (
            thickness > 0.0
            and math.isfinite(thickness)
            and conductivity > 0.0
            and math.isfinite(conductivity)
            and density > 0.0
            and math.isfinite(density)
            and heat_capacity > 0.0
            and math.isfinite(heat_capacity)
        ):
            raise InvalidInputError(
                "wall", "the class's constructor refuses a non-positive wall property"
            )
        if num_nodes < 3:
            raise InvalidInputError(
                "wall_nodes", "the class's constructor requires at least three nodes"
            )
        if not (initial_temperature > 0.0 and math.isfinite(initial_temperature)):
            raise InvalidInputError(
                "initial_temperature",
                "the class's constructor requires a positive initial temperature",
            )
        self.num_nodes = num_nodes
        self.total_thickness = thickness
        self.dx = thickness / (num_nodes - 1.0)
        self.temperature = [initial_temperature] * num_nodes
        self.conductivity = [conductivity] * num_nodes
        self.density = [density] * num_nodes
        self.heat_capacity = [heat_capacity] * num_nodes
        alpha = conductivity / (density * heat_capacity)
        if not (alpha > 0.0 and math.isfinite(alpha)):
            raise InvalidInputError(
                "wall", "the wall's thermal diffusivity is not positive and finite"
            )
        self.diffusivity = [alpha] * num_nodes

    def advance_time_step(
        self,
        dt: float,
        inner_fluid_temperature: float,
        inner_htc: float,
        outer_ambient_temperature: float,
        outer_htc: float,
        additional_outer_flux: float,
    ) -> None:
        """Advance the profile by one step, subdividing it where the scheme would be unstable."""
        if not (dt > 0.0 and math.isfinite(dt)):
            raise InvalidInputError("time_step", "the class refuses a non-positive time step")
        max_alpha = max(self.diffusivity)
        max_dt = 0.5 * self.dx * self.dx / max_alpha
        if not (max_dt > 0.0 and math.isfinite(max_dt)):
            raise InvalidInputError("wall", "the CFL step of the wall is not positive and finite")
        if dt > max_dt:
            sub_steps = math.ceil(dt / max_dt) + 1
            sub_dt = dt / sub_steps
            for _ in range(sub_steps):
                self._advance_time_step_internal(
                    sub_dt,
                    inner_fluid_temperature,
                    inner_htc,
                    outer_ambient_temperature,
                    outer_htc,
                    additional_outer_flux,
                )
        else:
            self._advance_time_step_internal(
                dt,
                inner_fluid_temperature,
                inner_htc,
                outer_ambient_temperature,
                outer_htc,
                additional_outer_flux,
            )

    def _advance_time_step_internal(
        self,
        dt: float,
        inner_fluid_temperature: float,
        inner_htc: float,
        outer_ambient_temperature: float,
        outer_htc: float,
        additional_outer_flux: float,
    ) -> None:
        next_temperature = [0.0] * self.num_nodes
        for i in range(1, self.num_nodes - 1):
            k_left = (
                2.0
                * self.conductivity[i]
                * self.conductivity[i - 1]
                / (self.conductivity[i] + self.conductivity[i - 1])
            )
            k_right = (
                2.0
                * self.conductivity[i]
                * self.conductivity[i + 1]
                / (self.conductivity[i] + self.conductivity[i + 1])
            )
            k_effective = (k_left + k_right) / 2.0
            alpha_effective = k_effective / (self.density[i] * self.heat_capacity[i])
            fourier = alpha_effective * dt / (self.dx * self.dx)
            next_temperature[i] = self.temperature[i] + fourier * (
                self.temperature[i - 1] - 2.0 * self.temperature[i] + self.temperature[i + 1]
            )

        k_inner = self.conductivity[0]
        biot_inner = inner_htc * self.dx / k_inner
        fourier_inner = self.diffusivity[0] * dt / (self.dx * self.dx)
        next_temperature[0] = self.temperature[0] + 2.0 * fourier_inner * (
            self.temperature[1]
            - self.temperature[0]
            + biot_inner * (inner_fluid_temperature - self.temperature[0])
        )

        n = self.num_nodes - 1
        k_outer = self.conductivity[n]
        biot_outer = outer_htc * self.dx / k_outer
        fourier_outer = self.diffusivity[n] * dt / (self.dx * self.dx)
        flux_term = additional_outer_flux * self.dx / k_outer
        next_temperature[n] = self.temperature[n] + 2.0 * fourier_outer * (
            self.temperature[n - 1]
            - self.temperature[n]
            + biot_outer * (outer_ambient_temperature - self.temperature[n])
            + flux_term
        )

        self.temperature = next_temperature

    def inner_wall_temperature(self) -> float:
        """The inner surface temperature, K."""
        return self.temperature[0]

    def outer_wall_temperature(self) -> float:
        """The outer surface temperature, K."""
        return self.temperature[self.num_nodes - 1]

    def mean_wall_temperature(self) -> float:
        """The unweighted node mean, K."""
        return sum(self.temperature) / self.num_nodes

    def heat_absorbed(self, wall_area: float, previous_mean_temperature: float) -> float:
        """The heat absorbed over a step, J."""
        delta = self.mean_wall_temperature() - previous_mean_temperature
        per_node = (
            sum(
                density * capacity
                for density, capacity in zip(self.density, self.heat_capacity, strict=True)
            )
            / self.num_nodes
        )
        return per_node * self.total_thickness * wall_area * delta
