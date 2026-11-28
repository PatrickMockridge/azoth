import neqsim.thermo.component.ComponentEosInterface;
import neqsim.thermo.component.attractiveeosterm.AttractiveTermCPAWaterCaloric;
import neqsim.thermo.system.SystemInterface;
import neqsim.thermo.system.SystemSrkEos;

// `AttractiveTermCPAWaterCaloric.alpha` for water across its calibration window and beyond.
//
//     javac -proc:none -cp neqsim-b40bb11.jar CpaWaterCaloricProbe.java
//     java -cp .:neqsim-b40bb11.jar CpaWaterCaloricProbe
//
// The class adds no algebra of its own: it passes five coefficients to
// `AttractiveTermMatCop5PRUMR` and keeps the parent's five-term polynomial.

public class CpaWaterCaloricProbe {
  public static void main(String[] args) {
    SystemInterface system = new SystemSrkEos(298.15, 1.0);
    system.addComponent("water", 1.0);
    system.init(0);
    ComponentEosInterface water = (ComponentEosInterface) system.getPhase(0).getComponent(0);

    System.out.printf("water_tc_k\t%.17g%n", water.getTC());
    AttractiveTermCPAWaterCaloric term = new AttractiveTermCPAWaterCaloric(water);
    double[] temperatures = {278.15, 298.15, 333.15, 373.15, 423.15};
    for (double temperature : temperatures) {
      System.out.printf("alpha_T_%.17g\t%.17g%n", temperature, term.alpha(temperature));
    }
  }
}
