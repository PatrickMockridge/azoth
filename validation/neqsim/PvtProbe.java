// The PVT probes. One subcommand per id; the states are the pinned tests' own where the
// class is expressible in the id's boundary units, and each answer is printed as the id
// publishes it.

import neqsim.pvtsimulation.util.BlackOilCorrelations;
import neqsim.pvtsimulation.util.BlackOilUnits;
import neqsim.pvtsimulation.util.SaturationPressureCorrelation;

public final class PvtProbe {

  private static void line(String key, double value) {
    System.out.println(key + "=" + value);
  }

  private static double psiaToPa(double psia) {
    return SaturationPressureCorrelation.psiaToBar(psia) * 1.0e5;
  }

  private static void blackOil() {
    // The class's test state: Rs = 500 scf/STB, gamma_g = 0.8, gamma_o = 0.85, API = 35,
    // T = 200 F. The id's boundary takes the SI conversions of those values.
    double rs = BlackOilUnits.fromScfPerStb(500.0, BlackOilUnits.SI);
    double tC = BlackOilUnits.fromFahrenheit(200.0, BlackOilUnits.SI);
    double tK = BlackOilUnits.fromFahrenheit(200.0, BlackOilUnits.NEQSIM);
    double pBara = BlackOilUnits.fromPsia(2000.0, BlackOilUnits.SI);
    double pPa = pBara * 1.0e5;
    double muOd = BlackOilUnits.fromCentipoise(5.0, BlackOilUnits.SI);
    double rhoG = BlackOilUnits.fromLbPerFt3(5.0, BlackOilUnits.SI);
    double coBara = BlackOilUnits.fromPerPsi(1.5e-5, BlackOilUnits.SI);

    System.out.println("black_oil");

    line("api_from_specific_gravity_0.85", BlackOilCorrelations.apiFromSpecificGravity(0.85));
    line("specific_gravity_from_api_35.0", BlackOilCorrelations.specificGravityFromAPI(35.0));

    line("bubble_point_standing",
        BlackOilCorrelations.bubblePointStandingSI(rs, 0.8, 35.0, tC) * 1.0e5);
    double rsField = BlackOilUnits.toScfPerStb(rs, BlackOilUnits.SI);
    double tField = BlackOilUnits.toFahrenheit(tC, BlackOilUnits.SI);
    line("bubble_point_standing_by_specific_gravity",
        BlackOilUnits.fromPsia(
            BlackOilCorrelations.bubblePointStanding(rsField, 0.8, 0.85, tField, false),
            BlackOilUnits.SI) * 1.0e5);
    line("bubble_point_vasquez_beggs",
        BlackOilCorrelations.bubblePointVasquezBeggs(rs, 0.8, 35.0, tC, BlackOilUnits.SI) * 1.0e5);
    line("bubble_point_glaso_by_specific_gravity",
        BlackOilCorrelations.bubblePointGlasoSI(rs, 0.8, 35.0, tC) * 1.0e5);

    line("bubble_point_standing_p_sat", psiaToPa(SaturationPressureCorrelation.standing(500.0, 0.8, 35.0, 200.0)));
    line("bubble_point_vasquez_beggs_p_sat",
        psiaToPa(SaturationPressureCorrelation.vasquezBeggs(500.0, 0.8, 35.0, 200.0)));
    line("bubble_point_glaso_p_sat", psiaToPa(SaturationPressureCorrelation.glaso(500.0, 0.8, 35.0, 200.0)));
    line("bubble_point_petrosky_farshad",
        psiaToPa(SaturationPressureCorrelation.petroskyFarshad(500.0, 0.8, 35.0, 200.0)));
    line("bubble_point_al_marhoun",
        psiaToPa(SaturationPressureCorrelation.alMarhoun(500.0, 0.8, 0.85, 200.0)));

    double[] stats = SaturationPressureCorrelation.estimateWithStatistics(500.0, 0.8, 35.0, 200.0);
    line("bubble_point_statistics_average", psiaToPa(stats[0]));
    line("bubble_point_statistics_minimum", psiaToPa(stats[1]));
    line("bubble_point_statistics_maximum", psiaToPa(stats[2]));
    line("bubble_point_statistics_std_dev", psiaToPa(stats[3]));

    line("solution_gor_standing", BlackOilCorrelations.solutionGORStandingSI(pBara, 0.8, 35.0, tC));
    line("solution_gor_vasquez_beggs",
        BlackOilCorrelations.solutionGORVasquezBeggs(pBara, 0.8, 35.0, tC, BlackOilUnits.SI));
    line("oil_fvf_standing", BlackOilCorrelations.oilFVFStandingSI(rs, 0.8, 0.85, tC));
    line("oil_fvf_vasquez_beggs",
        BlackOilCorrelations.oilFVFVasquezBeggs(rs, 0.8, 35.0, tC, BlackOilUnits.SI));
    line("oil_fvf_undersaturated",
        BlackOilCorrelations.oilFVFUndersaturated(1.35, coBara, pBara * 2.0, pBara * 1.25,
            BlackOilUnits.SI));
    line("oil_compressibility_vasquez_beggs",
        BlackOilCorrelations.oilCompressibilityVasquesBeggsS(rs, 0.8, 35.0, tC, pBara * 1.5)
            * 1.0e-5);

    line("dead_oil_viscosity_beggs_robinson",
        BlackOilCorrelations.deadOilViscosityBeggsRobinsonSI(35.0, tC));
    line("dead_oil_viscosity_glaso", BlackOilCorrelations.deadOilViscosityGlasoSI(35.0, tC));
    line("dead_oil_viscosity_kartoatmodjo",
        BlackOilCorrelations.deadOilViscosityKartoatmodjoSI(35.0, tC));
    line("saturated_oil_viscosity_beggs_robinson",
        BlackOilCorrelations.saturatedOilViscosityBeggsRobinsonSI(muOd, rs));
    line("saturated_oil_viscosity_kartoatmodjo",
        BlackOilCorrelations.saturatedOilViscosityKartoatmodjoSI(muOd, rs));
    line("undersaturated_oil_viscosity_vasquez_beggs",
        BlackOilCorrelations.undersaturatedOilViscosityVasquesBeggsS(muOd, pBara * 2.0, pBara * 1.25));
    line("undersaturated_oil_viscosity_bergman_sutton",
        BlackOilCorrelations.undersaturatedOilViscosityBergmanSuttonSI(muOd, pBara * 2.0,
            pBara * 1.25));

    line("gas_fvf", BlackOilCorrelations.gasFVFSI(pBara, tC, 0.85));
    line("gas_fvf_rb_per_mscf",
        BlackOilCorrelations.gasFVFrbPerMscf(2000.0,
            BlackOilUnits.toRankine(tK, BlackOilUnits.NEQSIM), 0.85));
    line("gas_viscosity_lee_gonzalez_eakin",
        BlackOilCorrelations.gasViscosityLeeGonzalezEakinSI(tC, rhoG, 20.0));

    // The clamp, and the two branches the formulas select on.
    System.out.println("black_oil_boundaries");
    line("bubble_point_standing_p_sat_clamp", psiaToPa(SaturationPressureCorrelation.standing(1.0, 0.8, 35.0, 200.0)));
    line("bubble_point_vasquez_beggs_api_30",
        BlackOilCorrelations.bubblePointVasquezBeggs(rs, 0.8, 30.0, tC, BlackOilUnits.SI) * 1.0e5);
    line("bubble_point_vasquez_beggs_api_30_1",
        BlackOilCorrelations.bubblePointVasquezBeggs(rs, 0.8, 30.1, tC, BlackOilUnits.SI) * 1.0e5);
    line("bubble_point_glaso_api_35",
        BlackOilCorrelations.bubblePointGlaso(rs, 0.8, 35.0, tC, BlackOilUnits.SI) * 1.0e5);
    line("bubble_point_glaso_api_30",
        BlackOilCorrelations.bubblePointGlaso(rs, 0.8, 30.0, tC, BlackOilUnits.SI) * 1.0e5);
  }

  public static void main(String[] args) {
    String which = args.length > 0 ? args[0] : "black_oil";
    switch (which) {
      case "black_oil":
        blackOil();
        break;
      default:
        System.err.println("unknown subcommand: " + which);
        System.exit(2);
    }
  }
}
