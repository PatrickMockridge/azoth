import neqsim.thermo.util.empiric.BukacekWaterInGas;

// `BukacekWaterInGas`, forward and inverse.
//
//     javac -proc:none -cp neqsim-b40bb11.jar BukacekProbe.java
//     java -cp .:neqsim-b40bb11.jar BukacekProbe
//
// `getWaterInGas` reads pressure in bar (its `PCwater` is 220.64) and returns a mole
// fraction. `waterDewPointTemperature` is the class's own fixed-step inverse: 273.15 K
// start, step `(newppm - x) * 1e5`, relative `1e-8`, 1,000 iterations, last iterate at the
// cap.

public class BukacekProbe {
  public static void main(String[] args) {
    double[][] states = {{255.15, 70.0}, {273.15, 5.0}, {288.15, 1.0}, {300.0, 100.0}};
    for (double[] state : states) {
      System.out.printf("water_in_gas_T_%.17g_P_%.17g\t%.17g%n", state[0], state[1],
          BukacekWaterInGas.getWaterInGas(state[0], state[1]));
    }

    double[][] dewPoints = {{20.0e-6, 70.0}, {100.0e-6, 10.0}, {500.0e-6, 50.0}};
    for (double[] state : dewPoints) {
      System.out.printf("dew_point_x_%.17g_P_%.17g\t%.17g%n", state[0], state[1],
          BukacekWaterInGas.waterDewPointTemperature(state[0], state[1]));
    }

    // The inverse's trajectory, because its step is large enough for the last iterate at the
    // 1,000 cap to depend on the arithmetic's last bit.
    double x = 500.0e-6;
    double pressure = 50.0;
    double temperature = 273.15;
    for (int iteration = 1; iteration <= 12; iteration++) {
      double ppm = BukacekWaterInGas.getWaterInGas(temperature, pressure);
      temperature -= (ppm - x) * 1e5;
      System.out.printf("dew_point_trace_iter_%d_T\t%.17g%n", iteration, temperature);
    }
  }
}
