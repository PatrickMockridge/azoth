import neqsim.thermo.mixingrule.EosMixingRulesInterface;
import neqsim.thermo.phase.PhaseInterface;
import neqsim.thermo.system.SystemInterface;
import neqsim.thermo.system.SystemUMRPRUEos;
import neqsim.thermodynamicoperations.ThermodynamicOperations;

/**
 * NeqSim's {@code SystemUMRPRUEos} at one state.
 *
 * <p>
 * The class's constructor sets {@code setBmixType(1)} and {@code attractiveTermNumber = 1}
 * before any mixing rule is installed. {@code setMixingRule} builds a fresh handler, so the
 * question is which co-volume the phase actually solves with; this prints the type before
 * and after that call, then the flash.
 *
 * <pre>
 * javac -proc:none -cp neqsim-f0c7436.jar UmrPruProbe.java
 * java -cp .:neqsim-f0c7436.jar UmrPruProbe [T_K P_bara name z ...]
 * </pre>
 */
public final class UmrPruProbe {

  private static final double R = 8.3144621;

  private UmrPruProbe() {}

  private static void print(String label, double value) {
    System.out.printf("%-34s %.15g%n", label, value);
  }

  public static void main(String[] args) {
    double t = args.length > 0 ? Double.parseDouble(args[0]) : 330.0;
    double pBar = args.length > 1 ? Double.parseDouble(args[1]) : 25.0;
    String[] names = {"methane", "n-butane"};
    double[] z = {0.6, 0.4};
    if (args.length > 2) {
      int count = (args.length - 2) / 2;
      names = new String[count];
      z = new double[count];
      for (int i = 0; i < count; i++) {
        names[i] = args[2 + 2 * i];
        z[i] = Double.parseDouble(args[3 + 2 * i]);
      }
    }

    SystemInterface system = new SystemUMRPRUEos(t, pBar);
    for (int i = 0; i < names.length; i++) {
      system.addComponent(names[i], z[i]);
    }

    PhaseInterface before = system.getPhase(0);
    EosMixingRulesInterface ruleBefore = (EosMixingRulesInterface) before.getMixingRule();
    System.out.printf("bmixType_before                  %d%n", ruleBefore.getBmixType());

    system.setMixingRule("HV", "UNIFAC_UMRPRU");
    system.init(0);
    system.init(1);
    system.init(2);

    PhaseInterface phase = system.getPhase(0);
    EosMixingRulesInterface rule = (EosMixingRulesInterface) phase.getMixingRule();
    int n = phase.getNumberOfComponents();
    System.out.printf("bmixType_after                   %d%n", rule.getBmixType());
    print("A_attraction", phase.getA());
    print("B_covolume", phase.getB());
    print("alpha_mix", phase.getA() / (phase.getNumberOfMolesInPhase() * phase.getB() * R * t));
    print("Z", phase.getZ());
    for (int i = 0; i < n; i++) {
      print("lnPhi[" + i + "]", Math.log(phase.getComponent(i).getFugacityCoefficient()));
    }

    ThermodynamicOperations operations = new ThermodynamicOperations(system);
    operations.TPflash();
    system.init(3);
    System.out.printf("flashPhases                      %d%n", system.getNumberOfPhases());
    for (int i = 0; i < system.getNumberOfPhases(); i++) {
      PhaseInterface flashed = system.getPhase(i);
      StringBuilder composition = new StringBuilder();
      for (int j = 0; j < system.getNumberOfComponents(); j++) {
        composition.append(String.format("%.15g ", flashed.getComponent(j).getx()));
      }
      System.out.printf("flashPhase %d %s beta=%.15g Z=%.15g x=%s%n", i, flashed.getType(),
          flashed.getBeta(), flashed.getZ(), composition.toString().trim());
      for (int j = 0; j < system.getNumberOfComponents(); j++) {
        System.out.printf("flashPhase %d lnPhi[%d]=%.15g%n", i, j,
            Math.log(flashed.getComponent(j).getFugacityCoefficient()));
      }
    }
  }
}
