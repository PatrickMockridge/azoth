// TransientPipeProbe - the drift-flux transient pipe at the pinned commit.
//
// TransientPipe marches a one-dimensional gas/liquid line with an explicit finite-volume
// drift-flux scheme.  The probe runs two short cases on the same 100 m, 100 mm, five-cell
// line: a flowing inlet at a constant mass flow, and a closed inlet (a blowdown) at a
// constant outlet pressure.  It prints the inlet stream's flashed phase state, the final
// per-section primitive state, the allocated pressure history, and the step and inventory
// counters.  The capture is the expected-value source for eos.transient_pipe.
//
//   ../../tools/gated.sh javac -proc:none -cp neqsim-b40bb11.jar TransientPipeProbe.java
//   ../../tools/gated.sh java -cp .:neqsim-b40bb11.jar TransientPipeProbe > captures/transient_pipe.tsv

import java.util.Arrays;
import neqsim.process.equipment.pipeline.twophasepipe.PipeSection;
import neqsim.process.equipment.pipeline.twophasepipe.TransientPipe;
import neqsim.process.equipment.stream.Stream;
import neqsim.thermo.system.SystemInterface;
import neqsim.thermo.system.SystemSrkEos;

public class TransientPipeProbe {

  static SystemInterface fluid() {
    SystemSrkEos fluid = new SystemSrkEos(300.0, 50.0);
    fluid.addComponent("methane", 0.9);
    fluid.addComponent("n-butane", 0.1);
    fluid.setMixingRule("classic");
    fluid.setMultiPhaseCheck(true);
    return fluid;
  }

  static TransientPipe pipe(SystemInterface fluid, double massFlow, double outletBara,
      TransientPipe.BoundaryCondition inlet, TransientPipe.BoundaryCondition outlet) {
    Stream stream = new Stream("inlet", fluid);
    stream.setFlowRate(massFlow, "kg/sec");
    stream.setTemperature(300.0, "K");
    stream.setPressure(50.0, "bara");
    stream.run();

    TransientPipe pipe = new TransientPipe("pipe", stream);
    pipe.setLength(100.0);
    pipe.setDiameter(0.1);
    pipe.setNumberOfSections(5);
    pipe.setRoughness(1.0e-4);
    pipe.setMaxSimulationTime(1.0);
    pipe.setCflNumber(0.5);
    pipe.setThermodynamicUpdateInterval(10);
    pipe.setInletBoundaryCondition(inlet);
    pipe.setOutletBoundaryCondition(outlet);
    pipe.setOutletPressure(outletBara);
    return pipe;
  }

  static String numbers(double[] values) {
    return Arrays.toString(values).replace("[", "").replace("]", "").replace(", ", " ");
  }

  static void inletRow(String label, TransientPipe pipe) {
    SystemInterface stream = pipe.getInletStream().getFluid();
    System.out.println("  " + label + " inlet_p_bara=" + stream.getPressure("bara")
        + " inlet_t_k=" + stream.getTemperature("K") + " inlet_mass_flow_kg_s="
        + pipe.getInletStream().getFlowRate("kg/sec") + " phases=" + stream.getNumberOfPhases()
        + " density_kg_m3=" + stream.getDensity("kg/m3") + " volume_m3=" + stream.getVolume());
    for (int i = 0; i < stream.getNumberOfPhases(); i++) {
      var phase = stream.getPhase(i);
      System.out.println("    phase=" + i + " type=" + phase.getPhaseTypeName() + " beta="
          + phase.getBeta() + " density_kg_m3=" + phase.getDensity("kg/m3") + " viscosity_pa_s="
          + phase.getViscosity("kg/msec") + " molar_mass_kg_mol=" + phase.getMolarMass()
          + " enthalpy_j_mol=" + phase.getEnthalpy("J/mol") + " sound_speed_m_s="
          + phase.getSoundSpeed() + " volume_m3=" + phase.getVolume());
    }
    stream.getInterphaseProperties().init(stream);
    System.out.println("    surface_tension_mn_m="
        + stream.getInterphaseProperties().getSurfaceTension(0, 1));
  }

  static void row(String label, TransientPipe pipe) {
    try {
      pipe.run();
    } catch (RuntimeException failure) {
      System.out.println("  " + label + " status=refused reason="
          + failure.getClass().getSimpleName() + ":" + failure.getMessage());
      return;
    }
    inletRow(label, pipe);
    System.out.println("  " + label + " simulation_time_s=" + pipe.getSimulationTime()
        + " total_time_steps=" + pipe.getTotalTimeSteps() + " mass_residual_kg="
        + pipe.getMassResidual() + " energy_residual_j=" + pipe.getEnergyResidual());
    double[][] history = pipe.getPressureHistory();
    System.out.println("    history_rows=" + history.length + " history_columns="
        + (history.length == 0 ? 0 : history[0].length));
    for (int i = 0; i < history.length; i++) {
      System.out.println("    history=" + i + " pressure_pa=" + numbers(history[i]));
    }
    System.out.println("    profile pressure_pa=" + numbers(pipe.getPressureProfile()));
    System.out.println("    profile temperature_k=" + numbers(pipe.getTemperatureProfile()));
    System.out.println("    profile liquid_holdup=" + numbers(pipe.getLiquidHoldupProfile()));
    System.out.println("    profile gas_velocity_m_s=" + numbers(pipe.getGasVelocityProfile()));
    System.out.println("    profile liquid_velocity_m_s=" + numbers(pipe.getLiquidVelocityProfile()));
    PipeSection[] sections = pipe.getSections();
    for (int i = 0; i < sections.length; i++) {
      PipeSection section = sections[i];
      System.out.println("    section=" + i + " pressure_pa=" + section.getPressure()
          + " temperature_k=" + section.getTemperature() + " liquid_holdup="
          + section.getLiquidHoldup() + " gas_holdup=" + section.getGasHoldup()
          + " gas_velocity_m_s=" + section.getGasVelocity() + " liquid_velocity_m_s="
          + section.getLiquidVelocity() + " mixture_velocity_m_s=" + section.getMixtureVelocity()
          + " gas_density_kg_m3=" + section.getGasDensity() + " liquid_density_kg_m3="
          + section.getLiquidDensity() + " gas_viscosity_pa_s=" + section.getGasViscosity()
          + " liquid_viscosity_pa_s=" + section.getLiquidViscosity() + " gas_sound_speed_m_s="
          + section.getGasSoundSpeed() + " liquid_sound_speed_m_s=" + section.getLiquidSoundSpeed()
          + " surface_tension_n_m=" + section.getSurfaceTension() + " flow_regime="
          + section.getFlowRegime());
    }
  }

  public static void main(String[] args) {
    System.out.println("transient_pipe");
    System.out.println("  case=flowing_inlet");
    row("flowing_inlet", pipe(fluid(), 5.0, 40.0, TransientPipe.BoundaryCondition.CONSTANT_FLOW,
        TransientPipe.BoundaryCondition.CONSTANT_PRESSURE));
    System.out.println("  case=closed_inlet");
    row("closed_inlet", pipe(fluid(), 5.0, 40.0, TransientPipe.BoundaryCondition.CLOSED,
        TransientPipe.BoundaryCondition.CONSTANT_PRESSURE));
  }
}
