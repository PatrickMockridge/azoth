import neqsim.thermo.component.ComponentEosInterface;
import neqsim.thermo.component.attractiveeosterm.AttractiveTermGERG;
import neqsim.thermo.system.SystemInterface;
import neqsim.thermo.system.SystemSrkEos;

// `AttractiveTermGERG.alpha` for water, on both sides of its 273.15 K switch.
//
//     javac -proc:none -cp neqsim-b40bb11.jar GergWaterAlphaProbe.java
//     java -cp .:neqsim-b40bb11.jar GergWaterAlphaProbe
//
// The class keeps two three-parameter sets, `parametersGERG` above 273.15 K and
// `parametersSolidGERG` below it, and `alpha()` copies one into `parameters` on every call.
// The probe prints the component's `getMatiascopemanParams` beside the class's own sets so
// the constructor's copy can be seen to be overwritten.

public class GergWaterAlphaProbe {
  public static void main(String[] args) {
    SystemInterface system = new SystemSrkEos(298.15, 1.0);
    system.addComponent("water", 1.0);
    system.init(0);
    ComponentEosInterface water = (ComponentEosInterface) system.getPhase(0).getComponent(0);

    System.out.printf("water_tc_k\t%.17g%n", water.getTC());
    print("component_matiascopeman", water.getMatiascopemanParams());
    print("component_matiascopeman_solid", water.getMatiascopemanSolidParams());

    AttractiveTermGERG term = new AttractiveTermGERG(water);
    double[] temperatures = {250.0, 273.149999, 273.15, 273.16, 300.0, 373.15, 473.15};
    for (double temperature : temperatures) {
      System.out.printf("alpha_T_%.17g\t%.17g%n", temperature, term.alpha(temperature));
    }
  }

  private static void print(String key, double[] values) {
    if (values == null) {
      System.out.println(key + "\tnull");
      return;
    }
    StringBuilder line = new StringBuilder(key);
    for (double value : values) {
      line.append('\t').append(String.format("%.17g", value));
    }
    System.out.println(line);
  }
}
