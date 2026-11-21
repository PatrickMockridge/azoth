// The PVT probes. One subcommand per id; the states are the pinned tests' own where the
// class is expressible in the id's boundary units, and each answer is printed as the id
// publishes it.

import java.util.Map;
import neqsim.pvtsimulation.reservoirproperties.CompositionEstimation;
import neqsim.pvtsimulation.simulation.SolutionGasWaterRatio;
import neqsim.pvtsimulation.util.BlackOilCorrelations;
import neqsim.pvtsimulation.util.DeclineCurveAnalysis;
import neqsim.pvtsimulation.util.BlackOilUnits;
import neqsim.pvtsimulation.util.GasPseudoCriticalProperties;
import neqsim.pvtsimulation.util.GasPseudoPressure;
import neqsim.pvtsimulation.util.SaturationPressureCorrelation;
import neqsim.pvtsimulation.util.WaterPropertyCorrelations;
import neqsim.pvtsimulation.util.ZFactorCorrelations;
import neqsim.thermo.system.SystemInterface;
import neqsim.thermo.system.SystemSrkEos;

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

  private static void gas() {
    System.out.println("gas_correlations");

    line("pseudo_critical_temperature_standing_0.75",
        GasPseudoCriticalProperties.pseudoCriticalTemperatureStanding(0.75));
    line("pseudo_critical_pressure_standing_0.75",
        GasPseudoCriticalProperties.pseudoCriticalPressureStanding(0.75) * 1.0e5);
    line("pseudo_critical_temperature_sutton_0.75",
        GasPseudoCriticalProperties.pseudoCriticalTemperatureSutton(0.75));
    line("pseudo_critical_pressure_sutton_0.75",
        GasPseudoCriticalProperties.pseudoCriticalPressureSutton(0.75) * 1.0e5);
    line("pseudo_critical_temperature_piper",
        GasPseudoCriticalProperties.pseudoCriticalTemperaturePiper(0.75, 0.05, 0.03, 0.02));
    line("pseudo_critical_pressure_piper",
        GasPseudoCriticalProperties.pseudoCriticalPressurePiper(0.75, 0.05, 0.03, 0.02) * 1.0e5);

    double[] corrected = GasPseudoCriticalProperties.wichertAzizCorrection(222.22, 45.51, 0.10, 0.05);
    line("wichert_aziz_temperature", corrected[0]);
    line("wichert_aziz_pressure", corrected[1] * 1.0e5);

    line("pseudo_reduced_temperature", GasPseudoCriticalProperties.pseudoReducedTemperature(366.48, 222.22));
    line("pseudo_reduced_pressure", GasPseudoCriticalProperties.pseudoReducedPressure(137.9, 45.5));

    line("z_factor_hall_yarborough_1.5_2.0", ZFactorCorrelations.hallYarborough(1.5, 2.0));
    line("z_factor_hall_yarborough_2.0_10.0", ZFactorCorrelations.hallYarborough(2.0, 10.0));
    line("z_factor_hall_yarborough_1.5_0.0", ZFactorCorrelations.hallYarborough(1.5, 0.0));
    line("z_factor_dranchuk_abou_kassem_1.5_2.0", ZFactorCorrelations.dranchukAbouKassem(1.5, 2.0));
    line("z_factor_dranchuk_abou_kassem_2.0_10.0", ZFactorCorrelations.dranchukAbouKassem(2.0, 10.0));
    line("z_factor_papay_1.5_2.0", ZFactorCorrelations.papay(1.5, 2.0));
    line("z_factor_papay_1.5_10.0", ZFactorCorrelations.papay(1.5, 10.0));
    line("z_factor_sutton", ZFactorCorrelations.zFactorSutton(137.9, 366.48, 0.65));
    line("z_factor_sour_gas", ZFactorCorrelations.zFactorSourGas(137.9, 366.48, 0.75, 0.10, 0.05));
    line("gas_density", ZFactorCorrelations.gasDensity(137.9, 366.48, 0.65, 16.04));
    line("gas_fvf_from_z", ZFactorCorrelations.gasFVFFromZ(137.9, 366.48, 0.65));

    line("pseudo_pressure_from_correlation",
        GasPseudoPressure.calculateFromCorrelation(206.84, 1.01325, 366.48, 0.65, 16.04));
    line("delta_pseudo_pressure",
        GasPseudoPressure.deltaPseudoPressure(206.84, 68.95, 366.48, 0.65, 16.04));

    CompositionEstimation composition = new CompositionEstimation(366.48, 206.84);
    line("h2s_concentration_from_temperature", composition.estimateH2Sconcentration());
    line("h2s_concentration_from_co2", composition.estimateH2Sconcentration(0.05));
  }

  private static void water() {
    double t = 366.48;
    double p = 206.84;
    double ppm = 30000.0;

    System.out.println("water_correlations");
    line("water_fvf_mccain", WaterPropertyCorrelations.waterFVFMcCain(t, p));
    line("water_fvf_osif", WaterPropertyCorrelations.waterFVFOsif(t, p, ppm));
    line("dead_water_viscosity_mccain",
        WaterPropertyCorrelations.deadWaterViscosityMcCain(t, ppm) * 1.0e-3);
    line("water_viscosity_mccain",
        WaterPropertyCorrelations.waterViscosityMcCain(t, p, ppm) * 1.0e-3);
    line("water_compressibility_mccain",
        WaterPropertyCorrelations.waterCompressibilityMcCain(t, p, ppm) * 1.0e-5);
    line("brine_density_batzle_wang", WaterPropertyCorrelations.brineDensityBatzleWang(t, p, 0.03));
    line("water_density", WaterPropertyCorrelations.waterDensity(t, p));
    line("solution_gas_water_ratio_culberson",
        WaterPropertyCorrelations.solutionGasWaterRatioCulberson(t, p, ppm));
    line("water_gas_surface_tension", WaterPropertyCorrelations.waterGasSurfaceTension(t, p) * 1.0e-3);

    SystemInterface gas = new SystemSrkEos(373.15, 200.0);
    gas.addComponent("methane", 0.85);
    gas.addComponent("ethane", 0.08);
    gas.addComponent("propane", 0.04);
    gas.addComponent("CO2", 0.03);
    gas.setMixingRule("classic");
    SolutionGasWaterRatio rsw = new SolutionGasWaterRatio(gas);
    rsw.setCalculationMethod(SolutionGasWaterRatio.CalculationMethod.MCCAIN);
    rsw.setTemperaturesAndPressures(new double[] {373.15}, new double[] {100.0});
    rsw.setSalinity(0.0);
    rsw.runCalc();
    line("rsw_mccain_pure", rsw.getRsw(0));
    rsw.setSalinity(3.5, "wt%");
    line("rsw_mccain_3_5_wt_molality", rsw.getSalinity());
    rsw.setTemperaturesAndPressures(new double[] {373.15}, new double[] {100.0});
    rsw.runCalc();
    line("rsw_mccain_3_5_wt", rsw.getRsw(0));
  }

  private static void decline() {
    double qi = 1000.0;
    double di = 0.001;
    System.out.println("decline_curve");

    line("rate_b0", DeclineCurveAnalysis.rate(qi, di, 0.0, 365.25));
    line("rate_b0.5", DeclineCurveAnalysis.rate(qi, di, 0.5, 365.25));
    line("rate_b1", DeclineCurveAnalysis.rate(qi, di, 1.0, 365.25));
    line("rate_exponential", DeclineCurveAnalysis.rateExponential(qi, di, 365.25));
    line("rate_hyperbolic", DeclineCurveAnalysis.rateHyperbolic(qi, di, 0.5, 365.25));
    line("rate_harmonic", DeclineCurveAnalysis.rateHarmonic(qi, di, 365.25));
    line("cumulative_b0", DeclineCurveAnalysis.cumulativeProduction(qi, di, 0.0, 3650.0));
    line("cumulative_b0.5", DeclineCurveAnalysis.cumulativeProduction(qi, di, 0.5, 3650.0));
    line("cumulative_b1", DeclineCurveAnalysis.cumulativeProduction(qi, di, 1.0, 3650.0));
    line("cumulative_exponential", DeclineCurveAnalysis.cumulativeExponential(qi, di, 3650.0));
    line("cumulative_harmonic", DeclineCurveAnalysis.cumulativeHarmonic(qi, di, 3650.0));
    line("nominal_to_effective", DeclineCurveAnalysis.nominalToEffectiveAnnual(di));
    line("effective_to_nominal",
        DeclineCurveAnalysis.effectiveAnnualToNominal(DeclineCurveAnalysis.nominalToEffectiveAnnual(di)));
    line("instantaneous_decline_rate", DeclineCurveAnalysis.instantaneousDeclineRate(di, 0.5, 1000.0));
    line("time_to_rate", DeclineCurveAnalysis.timeToRate(qi, di, 0.5, 50.0));
    line("eur", DeclineCurveAnalysis.eur(qi, di, 0.5, 50.0));
    line("remaining", DeclineCurveAnalysis.remainingReserves(qi, di, 0.5, 1000.0, 50.0));

    double[][] profile = DeclineCurveAnalysis.forecast(qi, di, 0.5, 0.0, 3650.0, 365.0);
    for (int i = 0; i < profile[0].length; i++) {
      line("forecast_time_" + i, profile[0][i]);
      line("forecast_rate_" + i, profile[1][i]);
      line("forecast_cumulative_" + i, profile[2][i]);
    }

    line("exponential_decline", DeclineCurveAnalysis.estimateExponentialDecline(1000.0, 0.0, 800.0, 365.0));
    Map<String, Double> hyperbolic =
        DeclineCurveAnalysis.estimateHyperbolicParameters(1000.0, 0.0, 700.0, 365.0, 500.0, 730.0);
    line("hyperbolic_qi", hyperbolic.get("qi"));
    line("hyperbolic_di", hyperbolic.get("di"));
    line("hyperbolic_b", hyperbolic.get("b"));

    double[] times = new double[11];
    double[] rates = new double[11];
    for (int i = 0; i < 11; i++) {
      times[i] = i * 365.0;
      rates[i] = qi * Math.exp(-di * times[i]);
    }
    Map<String, Double> fit = DeclineCurveAnalysis.fitArps(times, rates);
    line("fit_qi", fit.get("qi"));
    line("fit_di", fit.get("di"));
    line("fit_b", fit.get("b"));
    line("fit_r_squared", fit.get("rSquared"));
    line("eur_from_fit", DeclineCurveAnalysis.eurFromFit(fit, 50.0));

    Map<String, Double> window = DeclineCurveAnalysis.fitArps(times, rates, 2, 10);
    line("window_qi", window.get("qi"));
    line("window_di", window.get("di"));
    line("window_b", window.get("b"));
    line("window_r_squared", window.get("rSquared"));

    line("duong_time", DeclineCurveAnalysis.duongTimeFunction(0.5, 1.2, 365.0));
    line("duong_rate", DeclineCurveAnalysis.rateDuong(1000.0, 0.5, 1.2, 365.0));
    line("duong_cumulative", DeclineCurveAnalysis.cumulativeDuong(1000.0, 0.5, 1.2, 365.0));
    double[] duongTimes = new double[10];
    double[] duongRates = new double[10];
    for (int i = 0; i < 10; i++) {
      duongTimes[i] = (i + 1) * 365.0;
      duongRates[i] = DeclineCurveAnalysis.rateDuong(1000.0, 0.5, 1.2, duongTimes[i]);
    }
    Map<String, Double> duongFit = DeclineCurveAnalysis.fitDuong(duongTimes, duongRates);
    line("duong_fit_q1", duongFit.get("q1"));
    line("duong_fit_a", duongFit.get("a"));
    line("duong_fit_m", duongFit.get("m"));
    line("duong_fit_r_squared", duongFit.get("rSquared"));

    Map<String, Double> summary = DeclineCurveAnalysis.summary(qi, di, 0.5, 50.0);
    line("summary_effective", summary.get("effectiveAnnualDecline"));
    line("summary_rate_1yr", summary.get("rate_1yr"));
    line("summary_rate_5yr", summary.get("rate_5yr"));
    line("summary_rate_10yr", summary.get("rate_10yr"));
    line("summary_cum_1yr", summary.get("cumProd_1yr"));
    line("summary_cum_5yr", summary.get("cumProd_5yr"));
    line("summary_cum_10yr", summary.get("cumProd_10yr"));
    line("summary_time_to_limit", summary.get("timeToEconomicLimit_days"));
    line("summary_time_to_limit_years", summary.get("timeToEconomicLimit_years"));
    line("summary_eur", summary.get("EUR"));
  }

  public static void main(String[] args) {
    String which = args.length > 0 ? args[0] : "black_oil";
    switch (which) {
      case "black_oil":
        blackOil();
        break;
      case "gas":
        gas();
        break;
      case "water":
        water();
        break;
      case "decline":
        decline();
        break;
      default:
        System.err.println("unknown subcommand: " + which);
        System.exit(2);
    }
  }
}
