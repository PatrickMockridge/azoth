// The PVT probes. One subcommand per id; the states are the pinned tests' own where the
// class is expressible in the id's boundary units, and each answer is printed as the id
// publishes it.
//
// A few published diagnostics are private arrays in the classes and have no getter. The
// probe reads those by reflection, walking the superclass chain. That is probe surface, not
// class surface: the library never reflects, and a field rename at a later pin breaks this
// file rather than an id.

import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.Map;
import org.apache.commons.lang3.SerializationUtils;
import neqsim.pvtsimulation.modeltuning.TuneToSaturation;
import neqsim.pvtsimulation.regression.ExperimentType;
import neqsim.pvtsimulation.regression.PVTRegression;
import neqsim.pvtsimulation.regression.PVTRegressionFunction;
import neqsim.pvtsimulation.regression.RegressionParameter;
import neqsim.pvtsimulation.regression.RegressionParameterConfig;
import neqsim.pvtsimulation.regression.RegressionResult;
import neqsim.pvtsimulation.regression.UncertaintyAnalysis;
import neqsim.pvtsimulation.reservoirproperties.CompositionEstimation;
import neqsim.pvtsimulation.reservoirproperties.materialbalance.GasMaterialBalance;
import neqsim.pvtsimulation.reservoirproperties.materialbalance.OilMaterialBalance;
import neqsim.pvtsimulation.simulation.ConstantMassExpansion;
import neqsim.pvtsimulation.simulation.ConstantVolumeDepletion;
import neqsim.pvtsimulation.simulation.GOR;
import neqsim.pvtsimulation.simulation.MMPCalculator;
import neqsim.pvtsimulation.simulation.MultiStageSeparatorTest;
import neqsim.pvtsimulation.simulation.DensitySim;
import neqsim.pvtsimulation.simulation.DifferentialLiberation;
import neqsim.pvtsimulation.simulation.SaturationPressure;
import neqsim.pvtsimulation.simulation.SaturationTemperature;
import neqsim.pvtsimulation.simulation.SeparatorTest;
import neqsim.pvtsimulation.simulation.SwellingTest;
import neqsim.pvtsimulation.simulation.ViscositySim;
import neqsim.pvtsimulation.simulation.WaxFractionSim;
import neqsim.pvtsimulation.simulation.SolutionGasWaterRatio;
import neqsim.pvtsimulation.util.BlackOilCorrelations;
import neqsim.pvtsimulation.util.DeclineCurveAnalysis;
import neqsim.pvtsimulation.util.BlackOilUnits;
import neqsim.pvtsimulation.util.GasPseudoCriticalProperties;
import neqsim.pvtsimulation.util.GasPseudoPressure;
import neqsim.pvtsimulation.util.SaturationPressureCorrelation;
import neqsim.pvtsimulation.util.WaterPropertyCorrelations;
import neqsim.pvtsimulation.util.ZFactorCorrelations;
import neqsim.statistics.parameterfitting.SampleSet;
import neqsim.statistics.parameterfitting.SampleValue;
import neqsim.statistics.parameterfitting.nonlinearparameterfitting.LevenbergMarquardt;
import neqsim.statistics.parameterfitting.nonlinearparameterfitting.LevenbergMarquardtResult;
import neqsim.thermo.component.ComponentInterface;
import neqsim.thermo.system.SystemInterface;
import neqsim.thermo.system.SystemSrkEos;
import neqsim.thermodynamicoperations.ThermodynamicOperations;

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

  private static void gasMaterialBalance() {
    double[] pressure = {300.0, 250.0, 200.0, 150.0, 100.0};
    double[] z = {0.9, 0.88, 0.85, 0.82, 0.80};
    double[] gp = {0.0, 1.0e6, 2.0e6, 3.0e6, 4.0e6};
    double[] we = {0.0, 0.0, 0.0, 0.0, 0.0};
    double temperature = 350.0;
    System.out.println("gas_material_balance");

    line("bg", GasMaterialBalance.bg(300.0, 0.9, temperature));
    GasMaterialBalance.Result fit = GasMaterialBalance.fitVolumetric(pressure, z, gp);
    line("fit_ogip", fit.getOgip());
    line("fit_pi_over_zi", fit.getPiOverZi());
    line("fit_slope", fit.getSlope());
    line("fit_r_squared", fit.getRSquared());
    GasMaterialBalance.Result gravity =
        GasMaterialBalance.fitVolumetric(pressure, gp, temperature, 0.7);
    line("gravity_ogip", gravity.getOgip());
    line("gravity_pi_over_zi", gravity.getPiOverZi());
    line("gravity_slope", gravity.getSlope());
    line("gravity_r_squared", gravity.getRSquared());
    double[][] cole = GasMaterialBalance.colePlot(pressure, z, gp, temperature);
    for (int i = 0; i < cole[0].length; i++) {
      line("cole_gp_" + i, cole[0][i]);
      line("cole_f_over_eg_" + i, cole[1][i]);
    }
    GasMaterialBalance.Result havlena =
        GasMaterialBalance.fitHavlenaOdeh(pressure, z, gp, we, temperature);
    line("havlena_ogip", havlena.getOgip());
    line("havlena_pi_over_zi", havlena.getPiOverZi());
    line("havlena_slope", havlena.getSlope());
    line("havlena_r_squared", havlena.getRSquared());
  }

  private static void oilMaterialBalance() {
    System.out.println("oil_material_balance");
    line("withdrawal_f", OilMaterialBalance.withdrawalF(1.0e6, 500.0, 1.2, 300.0, 0.005, 1.0e5, 1.01));
    line("eo", OilMaterialBalance.eo(1.2, 1.3, 400.0, 300.0, 0.005));
    line("eg", OilMaterialBalance.eg(1.3, 0.005, 0.004));
    line("efw", OilMaterialBalance.efw(1.3, 0.2, 4.0e-5, 0.2, 3.0e-5, 50.0));

    double[] eo = {0.1, 0.2, 0.3};
    double[] depletion = {1.0e6, 2.0e6, 3.0e6};
    OilMaterialBalance.Result depletionFit = OilMaterialBalance.fitDepletionDrive(depletion, eo);
    line("depletion_ooip", depletionFit.getOoip());
    line("depletion_m", depletionFit.getM());
    line("depletion_r_squared", depletionFit.getRSquared());

    double[] eg = {0.015, 0.018, 0.03};
    double[] gasCap = {1.03e6, 2.036e6, 3.06e6};
    OilMaterialBalance.Result gasCapFit = OilMaterialBalance.fitGasCapDrive(gasCap, eo, eg);
    line("gas_cap_ooip", gasCapFit.getOoip());
    line("gas_cap_m", gasCapFit.getM());
    line("gas_cap_r_squared", gasCapFit.getRSquared());

    double[] we = {0.0, 1.0e5, 2.0e5};
    double[] water = {1.0e6, 2.101e6, 3.202e6};
    OilMaterialBalance.Result waterFit = OilMaterialBalance.fitWaterDrive(water, eo, we, 1.01);
    line("water_ooip", waterFit.getOoip());
    line("water_m", waterFit.getM());
    line("water_r_squared", waterFit.getRSquared());

    double[] indices = OilMaterialBalance.driveIndices(1.0e7, 0.2, 0.2, 0.02, 0.001, 1.0e5, 1.01, 3.0e6);
    line("ddi", indices[0]);
    line("sdi", indices[1]);
    line("wdi", indices[2]);
    line("edi", indices[3]);
  }

  // A databank binary at a stated state. The laboratory ids' cases are databank fluids
  // because a FluidComponent row cannot carry a TBP cut; the upstream TBP states are
  // recorded as owed, not retyped here.
  private static SystemInterface binary(String first, double firstMoles, String second,
      double secondMoles, double temperatureK, double pressureBara) {
    SystemInterface system = new SystemSrkEos(temperatureK, pressureBara);
    system.addComponent(first, firstMoles);
    system.addComponent(second, secondMoles);
    system.setMixingRule(2);
    return system;
  }

  private static SystemInterface pure(String name, double moles, double temperatureK,
      double pressureBara) {
    SystemInterface system = new SystemSrkEos(temperatureK, pressureBara);
    system.addComponent(name, moles);
    system.setMixingRule(2);
    return system;
  }

  private static void saturation() {
    System.out.println("saturation");

    System.out.println("[pressure_methane_nc10_323]");
    SaturationPressure pressure = new SaturationPressure(
        binary("methane", 0.70, "nC10", 0.30, 323.15, 50.0));
    pressure.setTemperature(323.15);
    line("saturation_pressure_pa", pressure.calcSaturationPressure() * 1.0e5);

    System.out.println("[pressure_methane_nbutane_275]");
    pressure = new SaturationPressure(
        binary("methane", 0.95, "n-butane", 0.05, 275.0, 50.0));
    pressure.setTemperature(275.0);
    line("saturation_pressure_pa", pressure.calcSaturationPressure() * 1.0e5);

    // Above methane's critical temperature no pressure brackets a two-phase region, so the
    // class answers the top of its grid.
    System.out.println("[pressure_fallback_methane_300]");
    pressure = new SaturationPressure(pure("methane", 1.0, 300.0, 50.0));
    pressure.setTemperature(300.0);
    line("saturation_pressure_pa", pressure.calcSaturationPressure() * 1.0e5);

    System.out.println("[temperature_methane_nc10_52bara]");
    SaturationTemperature temperature = new SaturationTemperature(
        binary("methane", 0.70, "nC10", 0.30, 300.0, 52.1));
    line("saturation_temperature_k", temperature.calcSaturationTemperature());

    System.out.println("[temperature_fallback_methane_100bara]");
    temperature = new SaturationTemperature(pure("methane", 1.0, 300.0, 100.0));
    line("saturation_temperature_k", temperature.calcSaturationTemperature());
  }

  // One private field, walking the superclass chain. Probe-only; see the header.
  private static Object field(Object target, String name) {
    Class<?> type = target.getClass();
    while (type != null) {
      try {
        java.lang.reflect.Field found = type.getDeclaredField(name);
        found.setAccessible(true);
        return found.get(target);
      } catch (NoSuchFieldException absent) {
        type = type.getSuperclass();
      } catch (IllegalAccessException denied) {
        throw new IllegalStateException(denied);
      }
    }
    throw new IllegalStateException("no field " + name + " on " + target.getClass().getName());
  }

  private static double[] array(Object target, String name) {
    return (double[]) field(target, name);
  }

  private static double scalar(Object target, String name) {
    return (Double) field(target, name);
  }

  private static void value(String key, double value) {
    if (Double.isFinite(value)) {
      line(key, value);
    }
  }

  private static void cceBlock(String block, String first, double firstMoles, String second,
      double secondMoles, double temperatureK, double[] pressuresBar) {
    System.out.println("[" + block + "]");
    ConstantMassExpansion expansion = new ConstantMassExpansion(
        binary(first, firstMoles, second, secondMoles, temperatureK, pressuresBar[0]));
    expansion.setTemperature(temperatureK);
    expansion.setPressures(pressuresBar);
    expansion.runCalc();

    line("saturation_pressure_pa", expansion.getSaturationPressure() * 1.0e5);
    line("saturation_volume_m3", scalar(expansion, "saturationVolume") / 1.0e5);
    line("z_saturation", scalar(expansion, "Zsaturation"));
    line("saturation_isothermal_compressibility",
        expansion.getSaturationIsoThermalCompressibility() / 1.0e5);

    double[] total = array(expansion, "totalVolume");
    double[] gas = array(expansion, "gasVolume");
    double[] gasStandard = array(expansion, "gasStandardVolume");
    double[] relative = expansion.getRelativeVolume();
    double[] liquidRelative = expansion.getLiquidRelativeVolume();
    double[] yFactor = expansion.getYfactor();
    double[] density = expansion.getDensity();
    double[] viscosity = expansion.getViscosity();
    double[] oilDensity = expansion.getOilDensity();
    double[] oilViscosity = expansion.getOilViscosity();
    double[] bg = expansion.getBg();
    double[] zGas = expansion.getZgas();
    double[] compressibility = expansion.getIsoThermalCompressibility();
    for (int i = 0; i < pressuresBar.length; i++) {
      String at = "point_" + i + "_";
      line(at + "pressure_pa", pressuresBar[i] * 1.0e5);
      line(at + "total_volume_m3", total[i] / 1.0e5);
      line(at + "relative_volume", relative[i]);
      line(at + "density", density[i]);
      line(at + "gas_volume_m3", gas[i]);
      line(at + "gas_standard_volume_m3", gasStandard[i] / 1.0e5);
      line(at + "bg", bg[i]);
      line(at + "z_gas", zGas[i]);
      value(at + "oil_density", oilDensity[i]);
      value(at + "oil_viscosity", oilViscosity[i]);
      value(at + "liquid_relative_volume", liquidRelative[i] / 100.0);
      value(at + "y_factor", yFactor[i]);
      value(at + "viscosity", viscosity[i]);
      value(at + "isothermal_compressibility", compressibility[i] / 1.0e5);
    }
  }

  private static void cce() {
    System.out.println("cce");
    cceBlock("cce_methane_nc10_323", "methane", 0.70, "nC10", 0.30, 323.15,
        new double[] {300.0, 250.0, 200.0, 150.0, 100.0, 50.0, 10.0});
    cceBlock("cce_unsaturate_323", "methane", 0.70, "nC10", 0.30, 323.15,
        new double[] {400.0, 350.0, 300.0, 250.0});
    cceBlock("cce_methane_nbutane_275", "methane", 0.95, "n-butane", 0.05, 275.0,
        new double[] {100.0, 50.0, 20.0, 10.0, 5.0});
  }

  private static void dleBlock(String block, String first, double firstMoles, String second,
      double secondMoles, double temperatureK, double[] pressuresBar) {
    System.out.println("[" + block + "]");
    DifferentialLiberation liberation = new DifferentialLiberation(
        binary(first, firstMoles, second, secondMoles, temperatureK, pressuresBar[0]));
    liberation.setTemperature(temperatureK);
    liberation.setPressures(pressuresBar);
    liberation.runCalc();

    line("saturation_pressure_pa", liberation.getSaturationPressure() * 1.0e5);
    line("saturation_volume_m3", scalar(liberation, "saturationVolume"));
    line("voil_std_m3", scalar(liberation, "VoilStd"));

    double[] total = array(liberation, "totalVolume");
    double[] liquid = array(liberation, "liquidVolume");
    double[] liquidRelative = array(liberation, "liquidVolumeRelativeToVsat");
    double[] gas = array(liberation, "gasVolume");
    double[] relative = liberation.getRelativeVolume();
    double[] bo = liberation.getBo();
    double[] bg = liberation.getBg();
    double[] rs = liberation.getRs();
    double[] zGas = liberation.getZgas();
    double[] gasGravity = liberation.getRelGasGravity();
    double[] gasStandard = liberation.getGasStandardVolume();
    double[] oilDensity = liberation.getOilDensity();
    for (int i = 0; i < pressuresBar.length; i++) {
      String at = "point_" + i + "_";
      line(at + "pressure_pa", pressuresBar[i] * 1.0e5);
      line(at + "total_volume_m3", total[i]);
      line(at + "relative_volume", relative[i]);
      line(at + "liquid_volume_m3", liquid[i]);
      line(at + "oil_density", oilDensity[i]);
      if (gasStandard[i] > 1.0e-30) {
        line(at + "liquid_volume_relative_to_vsat", liquidRelative[i]);
        line(at + "gas_volume_m3", gas[i]);
        line(at + "gas_standard_volume_m3", gasStandard[i]);
        line(at + "z_gas", zGas[i]);
        line(at + "relative_gas_gravity", gasGravity[i]);
      }
      line(at + "bo", bo[i]);
      if (zGas[i] > 1.0e-10) {
        line(at + "bg", bg[i]);
      }
      line(at + "rs", rs[i]);
    }
  }

  private static void dle() {
    System.out.println("dle");
    dleBlock("dle_methane_nc10_323", "methane", 0.70, "nC10", 0.30, 323.15,
        new double[] {250.0, 200.0, 150.0, 100.0, 70.0, 50.0, 30.0, 20.0, 10.0, 1.0});
    dleBlock("dle_methane_nc10_350", "methane", 0.70, "nC10", 0.30, 350.0,
        new double[] {300.0, 250.0, 200.0, 150.0, 100.0, 50.0, 20.0, 5.0});
    dleBlock("dle_methane_nc10_323_rich", "methane", 0.60, "nC10", 0.40, 323.15,
        new double[] {250.0, 200.0, 150.0, 100.0, 50.0, 20.0, 5.0});
  }

  private static void cvdBlock(String block, String first, double firstMoles, String second,
      double secondMoles, double temperatureK, double[] pressuresBar) {
    System.out.println("[" + block + "]");
    ConstantVolumeDepletion depletion = new ConstantVolumeDepletion(
        binary(first, firstMoles, second, secondMoles, temperatureK, pressuresBar[0]));
    depletion.setTemperature(temperatureK);
    depletion.setPressures(pressuresBar);
    depletion.runCalc();

    line("saturation_pressure_pa", depletion.getSaturationPressure() * 1.0e5);
    line("saturation_volume_m3", scalar(depletion, "saturationVolume"));
    line("z_saturation", scalar(depletion, "Zsaturation"));
    line("initial_moles", scalar(depletion, "initialMoles"));
    line("cumulative_moles_removed", scalar(depletion, "cumulativeMolesRemoved"));

    double[] total = array(depletion, "totalVolume");
    double[] liquid = array(depletion, "liquidVolume");
    double[] liquidRelative = array(depletion, "liquidVolumeRelativeToVsat");
    double[] balance = array(depletion, "materialBalanceRelativeError");
    double[] relative = depletion.getRelativeVolume();
    double[] zmix = depletion.getZmix();
    double[] zGas = depletion.getZgas();
    double[] depleted = depletion.getCummulativeMolePercDepleted();
    double[] dropout = depletion.getLiquidDropoutCurve();
    for (int i = 0; i < pressuresBar.length; i++) {
      String at = "point_" + i + "_";
      line(at + "pressure_pa", pressuresBar[i] * 1.0e5);
      line(at + "total_volume_m3", total[i]);
      line(at + "relative_volume", relative[i]);
      line(at + "cumulative_mole_fraction_depleted", depleted[i] / 100.0);
      line(at + "material_balance_relative_error", balance[i]);
      line(at + "zmix", zmix[i]);
      if (zGas[i] > 1.0e-10) {
        line(at + "z_gas", zGas[i]);
      }
      if (liquid[i] > 0.0) {
        line(at + "liquid_volume_m3", liquid[i]);
        line(at + "liquid_volume_relative_to_vsat", liquidRelative[i]);
        line(at + "liquid_dropout", dropout[i] / 100.0);
      }
    }
  }

  private static void cvd() {
    System.out.println("cvd");
    cvdBlock("cvd_methane_nc10_323", "methane", 0.70, "nC10", 0.30, 323.15,
        new double[] {400.0, 350.0, 300.0, 250.0, 200.0, 150.0, 100.0, 50.0});
    cvdBlock("cvd_methane_nc10_350", "methane", 0.70, "nC10", 0.30, 350.0,
        new double[] {350.0, 300.0, 250.0, 200.0, 150.0, 100.0, 50.0});
    cvdBlock("cvd_rich_323", "methane", 0.60, "nC10", 0.40, 323.15,
        new double[] {300.0, 250.0, 200.0, 150.0, 100.0, 50.0});
  }


  private static SystemInterface multiStage() {
    SystemInterface system = new SystemSrkEos(373.15, 300.0);
    system.addComponent("nitrogen", 0.5);
    system.addComponent("CO2", 2.0);
    system.addComponent("methane", 45.0);
    system.addComponent("ethane", 8.0);
    system.addComponent("propane", 5.0);
    system.addComponent("i-butane", 1.5);
    system.addComponent("n-butane", 2.5);
    system.addComponent("i-pentane", 1.0);
    system.addComponent("n-pentane", 1.5);
    system.addComponent("n-hexane", 3.0);
    system.addComponent("n-heptane", 30.0);
    system.setMixingRule(2);
    system.setMultiPhaseCheck(true);
    return system;
  }

  private static MultiStageSeparatorTest train() {
    MultiStageSeparatorTest run = new MultiStageSeparatorTest(multiStage());
    run.setReservoirConditions(300.0, 100.0);
    run.addSeparatorStage(50.0, 40.0);
    run.addSeparatorStage(10.0, 30.0);
    run.addStockTankStage();
    return run;
  }

  private static void stageRows(MultiStageSeparatorTest run) {
    for (MultiStageSeparatorTest.SeparatorStageResult stage : run.getStageResults()) {
      String at = "stage_" + stage.getStageNumber() + "_";
      line(at + "pressure_pa", stage.getPressure() * 1.0e5);
      line(at + "temperature_k", stage.getTemperature() + 273.15);
      line(at + "stage_gor", stage.getStageGOR());
      line(at + "cumulative_gor", stage.getCumulativeGOR());
      value(at + "gas_volume_m3", stage.getGasRate() / 1.0e5);
      value(at + "gas_density", stage.getGasDensity());
      value(at + "oil_density", stage.getOilDensity());
      value(at + "gas_molar_mass", stage.getGasMW() / 1000.0);
      value(at + "oil_molar_mass", stage.getOilMW() / 1000.0);
      value(at + "gas_z_factor", stage.getGasZFactor());
      value(at + "oil_viscosity", stage.getOilViscosity() / 1000.0);
    }
  }

  private static void separator() {
    System.out.println("separator");
    double[] temperatures = {323.15, 313.15, 303.15, 288.15};
    double[] pressures = {50.0, 20.0, 5.0, 1.01325};

    System.out.println("[continuous_binary]");
    GOR gor = new GOR(binary("methane", 0.70, "nC10", 0.30, 323.15, 50.0));
    gor.setTemperaturesAndPressures(temperatures, pressures);
    gor.runCalc();
    for (int i = 0; i < pressures.length; i++) {
      String at = "stage_" + (i + 1) + "_";
      line(at + "pressure_pa", pressures[i] * 1.0e5);
      line(at + "temperature_k", temperatures[i]);
      line(at + "stage_gor", gor.getGOR()[i]);
      line(at + "oil_fvf", gor.getBofactor()[i]);
    }

    System.out.println("[staged_binary]");
    SeparatorTest staged = new SeparatorTest(binary("methane", 0.70, "nC10", 0.30, 323.15, 50.0));
    staged.setSeparatorConditions(temperatures, pressures);
    staged.runCalc();
    for (int i = 0; i < pressures.length; i++) {
      String at = "stage_" + (i + 1) + "_";
      line(at + "pressure_pa", pressures[i] * 1.0e5);
      line(at + "temperature_k", temperatures[i]);
      line(at + "stage_gor", staged.getGOR()[i]);
      line(at + "oil_fvf", staged.getBofactor()[i]);
    }

    System.out.println("[train_upstream]");
    MultiStageSeparatorTest run = train();
    run.run();
    line("total_gor", run.getTotalGOR());
    line("bo", run.getBo());
    line("rs", run.getRs());
    line("stock_tank_oil_density", run.getStockTankOilDensity());
    value("stock_tank_api", run.getStockTankAPIGravity());
    stageRows(run);

    System.out.println("[train_optimize]");
    MultiStageSeparatorTest optimized = train();
    optimized.run();
    MultiStageSeparatorTest.OptimizationResult best =
        optimized.optimizeFirstStageSeparator(5.0, 20.0, 3, 20.0, 40.0, 2);
    line("optimal_stage_pressure_pa", best.getOptimalPressure() * 1.0e5);
    line("optimal_stage_temperature_k", best.getOptimalTemperature() + 273.15);
    line("maximum_oil_recovery", best.getMaximumOilRecovery());
    line("gor_at_optimum", best.getGorAtOptimum());
    line("bo_at_optimum", best.getBoAtOptimum());
    value("api_at_optimum", best.getApiAtOptimum());
  }

  private static void swellingBlock(String block, String[] oilNames, double[] oilMoles,
      String[] gasNames, double[] gasMoles, double temperatureK, double[] schedule) {
    System.out.println("[" + block + "]");
    SystemInterface oil = new SystemSrkEos(temperatureK, 50.0);
    for (int i = 0; i < oilNames.length; i++) oil.addComponent(oilNames[i], oilMoles[i]);
    oil.setMixingRule(1);
    oil.useVolumeCorrection(false);
    SystemInterface gas = new SystemSrkEos(temperatureK, 50.0);
    for (int i = 0; i < gasNames.length; i++) gas.addComponent(gasNames[i], gasMoles[i]);
    gas.setMixingRule(1);
    gas.useVolumeCorrection(false);

    SwellingTest swelling = new SwellingTest(oil);
    swelling.setInjectionGas(gas);
    swelling.setCummulativeMolePercentGasInjected(schedule);
    swelling.setTemperature(temperatureK);
    swelling.runCalc();
    double[] pressures = swelling.getPressures();
    double[] volumes = swelling.getRelativeOilVolume();
    for (int i = 0; i < schedule.length; i++) {
      String at = "point_" + i + "_";
      line(at + "cumulative_gas_injected", schedule[i] / 100.0);
      line(at + "pressure_pa", pressures[i] * 1.0e5);
      line(at + "relative_oil_volume", volumes[i]);
    }
  }

  private static void swelling() {
    System.out.println("swelling");
    swellingBlock("swelling_upstream", new String[] {"methane", "nC10"}, new double[] {0.2, 0.8},
        new String[] {"methane"}, new double[] {1.0}, 353.15, new double[] {0.0, 10.0, 20.0});
    swellingBlock("swelling_mixed_gas", new String[] {"methane", "nC10"}, new double[] {0.5, 0.5},
        new String[] {"methane", "ethane"}, new double[] {0.7, 0.3}, 323.15,
        new double[] {0.0, 5.0, 15.0, 25.0});
    swellingBlock("swelling_zero_injection", new String[] {"methane", "nC10"},
        new double[] {0.5, 0.5}, new String[] {"methane"}, new double[] {1.0}, 323.15,
        new double[] {0.0, 0.0});
  }

  private static void mmpBlock(String block, double temperatureCelsius, String method,
      double minBar, double maxBar, int points, int nodes, double pv, double threshold) {
    mmpBlock(block, "methane", temperatureCelsius, method, minBar, maxBar, points, nodes, pv, threshold);
  }

  private static void mmpBlock(String block, String gasName, double temperatureCelsius,
      String method, double minBar, double maxBar, int points, int nodes, double pv,
      double threshold) {
    System.out.println("[" + block + "]");
    SystemInterface oil = new SystemSrkEos(temperatureCelsius + 273.15, minBar);
    oil.addComponent("methane", 0.70);
    oil.addComponent("nC10", 0.30);
    oil.setMixingRule(2);
    SystemInterface gas = new SystemSrkEos(temperatureCelsius + 273.15, minBar);
    gas.addComponent(gasName, 1.0);
    gas.setMixingRule(2);
    MMPCalculator calculator = new MMPCalculator(oil, gas);
    calculator.setTemperature(temperatureCelsius);
    calculator.setMethod(MMPCalculator.CalculationMethod.valueOf(method));
    calculator.setPressureRange(minBar, maxBar);
    calculator.setNumberOfPressurePoints(points);
    calculator.setSlimTubeParameters(nodes, pv);
    calculator.setRecoveryThreshold(threshold);
    calculator.run();
    line("mmp_pa", calculator.getMMP() * 1.0e5);
    System.out.println("mechanism=" + calculator.getMiscibilityMechanism().name());
    double[] pressures = calculator.getPressures();
    double[] recoveries = calculator.getRecoveries();
    if (pressures != null) {
      for (int i = 0; i < pressures.length; i++) {
        line("point_" + i + "_pressure_pa", pressures[i] * 1.0e5);
        line("point_" + i + "_recovery", recoveries[i]);
      }
    }
  }

  private static void mmp() {
    System.out.println("mmp");
    mmpBlock("mmp_slim_tube_small", 50.0, "SLIM_TUBE", 100.0, 300.0, 3, 6, 0.4, 0.90);
    mmpBlock("mmp_key_tie_line", 50.0, "KEY_TIE_LINE", 100.0, 500.0, 3, 6, 0.4, 0.90);
    mmpBlock("mmp_rising_bubble", 50.0, "RISING_BUBBLE", 100.0, 300.0, 3, 6, 0.4, 0.90);
    mmpBlock("mmp_default_grid", 50.0, "SLIM_TUBE", 100.0, 300.0, 10, 8, 0.5, 0.90);
    mmpBlock("mmp_slim_tube_wide", "methane", 50.0, "SLIM_TUBE", 100.0, 1000.0, 5, 6, 0.4, 0.50);
  }

  private static void propertySweep() {
    System.out.println("property_sweep");
    double[] temperatures = {323.15, 313.15, 303.15};
    double[] pressures = {5.0, 5.0, 5.0};

    System.out.println("[density_binary]");
    DensitySim density = new DensitySim(binary("methane", 0.70, "nC10", 0.30, 323.15, 5.0));
    density.setTemperaturesAndPressures(temperatures, pressures);
    density.runCalc();
    for (int i = 0; i < temperatures.length; i++) {
      String at = "point_" + i + "_";
      line(at + "temperature_k", temperatures[i]);
      line(at + "pressure_pa", pressures[i] * 1.0e5);
      if (density.getGasDensity()[i] > 0.0) {
        line(at + "gas_density", density.getGasDensity()[i]);
      }
      if (density.getOilDensity()[i] > 0.0) {
        line(at + "oil_density", density.getOilDensity()[i]);
      }
      if (density.getAqueousDensity()[i] > 0.0) {
        line(at + "aqueous_density", density.getAqueousDensity()[i]);
      }
    }

    System.out.println("[density_gas_binary]");
    DensitySim gasDensity = new DensitySim(pure("methane", 1.0, 300.0, 5.0));
    double[] gasTemperatures = {300.0, 290.0, 280.0};
    double[] gasPressures = {5.0, 5.0, 5.0};
    gasDensity.setTemperaturesAndPressures(gasTemperatures, gasPressures);
    gasDensity.runCalc();
    for (int i = 0; i < gasTemperatures.length; i++) {
      String at = "point_" + i + "_";
      line(at + "temperature_k", gasTemperatures[i]);
      line(at + "pressure_pa", gasPressures[i] * 1.0e5);
      if (gasDensity.getGasDensity()[i] > 0.0) {
        line(at + "gas_density", gasDensity.getGasDensity()[i]);
      }
      if (gasDensity.getOilDensity()[i] > 0.0) {
        line(at + "oil_density", gasDensity.getOilDensity()[i]);
      }
      if (gasDensity.getAqueousDensity()[i] > 0.0) {
        line(at + "aqueous_density", gasDensity.getAqueousDensity()[i]);
      }
    }

    System.out.println("[viscosity_binary]");
    ViscositySim viscosity = new ViscositySim(binary("methane", 0.70, "nC10", 0.30, 323.15, 5.0));
    viscosity.setTemperaturesAndPressures(temperatures, pressures);
    viscosity.runCalc();
    for (int i = 0; i < temperatures.length; i++) {
      String at = "point_" + i + "_";
      line(at + "temperature_k", temperatures[i]);
      line(at + "pressure_pa", pressures[i] * 1.0e5);
      if (viscosity.getGasViscosity()[i] > 0.0) {
        line(at + "gas_viscosity", viscosity.getGasViscosity()[i]);
      }
      if (viscosity.getOilViscosity()[i] > 0.0) {
        line(at + "oil_viscosity", viscosity.getOilViscosity()[i]);
      }
      line(at + "gas_phase_available", viscosity.getGasPhaseAvailable()[i] ? 1.0 : 0.0);
      line(at + "oil_phase_available", viscosity.getOilPhaseAvailable()[i] ? 1.0 : 0.0);
    }

    System.out.println("[viscosity_gas_binary]");
    ViscositySim gasViscosity = new ViscositySim(pure("methane", 1.0, 300.0, 5.0));
    gasViscosity.setTemperaturesAndPressures(gasTemperatures, gasPressures);
    gasViscosity.runCalc();
    for (int i = 0; i < gasTemperatures.length; i++) {
      String at = "point_" + i + "_";
      line(at + "temperature_k", gasTemperatures[i]);
      line(at + "pressure_pa", gasPressures[i] * 1.0e5);
      if (gasViscosity.getGasViscosity()[i] > 0.0) {
        line(at + "gas_viscosity", gasViscosity.getGasViscosity()[i]);
      }
      if (gasViscosity.getOilViscosity()[i] > 0.0) {
        line(at + "oil_viscosity", gasViscosity.getOilViscosity()[i]);
      }
      line(at + "gas_phase_available", gasViscosity.getGasPhaseAvailable()[i] ? 1.0 : 0.0);
      line(at + "oil_phase_available", gasViscosity.getOilPhaseAvailable()[i] ? 1.0 : 0.0);
    }
  }

  // ---- regression and tuning ---------------------------------------------------------------

  private static String number(double value) {
    return Double.isFinite(value) ? String.format(java.util.Locale.ROOT, "%.17g", value) : "-";
  }

  // One component as a row, the CharacterizationFluidRebuildProbe shape. A pseudo row carries
  // every property; a base row carries none.
  private static void componentRows(String prefix, SystemInterface system) {
    for (int i = 0; i < system.getNumberOfComponents(); i++) {
      ComponentInterface component = system.getComponent(i);
      if (!(component.getNumberOfmoles() > 0.0)) {
        continue;
      }
      boolean isPseudo = component.isIsTBPfraction() || component.isIsPlusFraction();
      StringBuilder row = new StringBuilder();
      row.append(component.getComponentName()).append('|').append(isPseudo).append('|')
          .append(number(component.getNumberOfmoles()));
      if (!isPseudo) {
        for (int k = 0; k < 19; k++) {
          row.append("|-");
        }
      } else {
        row.append('|').append(number(component.getMolarMass()));
        row.append('|').append(number(component.getNormalLiquidDensity() * 1000.0));
        row.append('|').append(number(component.getNormalBoilingPoint()));
        row.append('|').append(number(component.getTC()));
        row.append('|').append(number(component.getPC() * 1.0e5));
        row.append('|').append(number(component.getAcentricFactor()));
        row.append('|').append(number(component.getCriticalVolume() / 1.0e6));
        row.append('|').append(number(component.getRacketZ()));
        row.append('|').append(number(component.getRacketZCPA()));
        row.append('|').append(number(component.getParachorParameter()));
        row.append('|').append(number(component.getCriticalViscosity()));
        row.append('|').append(number(component.getTriplePointTemperature()));
        row.append('|').append(number(component.getHeatOfFusion()));
        row.append('|').append(number(component.getIdealGasEnthalpyOfFormation()));
        row.append('|').append(number(component.getCpA()));
        row.append('|').append(number(component.getCpB()));
        row.append('|').append(number(component.getCpC()));
        row.append('|').append(number(component.getCpD()));
        row.append('|').append(component.getAttractiveTerm() == null ? "-"
            : number(component.getAttractiveTerm().getm()));
      }
      System.out.println(prefix + i + " = " + row);
    }
  }

  // The class's own viscosity test fluid, rebuilt from PVTRegressionTest.createViscosityFluid.
  private static SystemInterface viscosityFluid() {
    SystemInterface fluid = new SystemSrkEos(320.0, 1.0);
    fluid.addComponent("n-heptane", 1.0);
    fluid.setMixingRule("classic");
    ThermodynamicOperations ops = new ThermodynamicOperations(fluid);
    ops.TPflash();
    fluid.getPhase("oil").getPhysicalProperties().setViscosityModel("PFCT");
    fluid.initProperties();
    return fluid;
  }

  // PVTRegressionTest.calculateOilViscosity: the third CSP factor at one state.
  private static double oilViscosity(SystemInterface fluid, double pressure, double temperature,
      double cspFactor) {
    fluid.setPressure(pressure);
    fluid.setTemperature(temperature);
    ThermodynamicOperations ops = new ThermodynamicOperations(fluid);
    ops.TPflash();
    fluid.getPhase("oil").getPhysicalProperties().setViscosityModel("PFCT");
    fluid.getPhase("oil").getPhysicalProperties().setCspViscosityCorrectionFactor(2, cspFactor);
    fluid.initProperties();
    return fluid.getPhase("oil").getPhysicalProperties().getViscosity();
  }

  // A TBP fluid: methane, n-decane and one C15 pseudo row. The row printer above carries the
  // pseudo component's twenty properties into the id's FluidComponent boundary.
  private static SystemInterface tbpFluid() {
    SystemInterface fluid = new SystemSrkEos(373.15, 250.0);
    fluid.addComponent("methane", 0.60);
    fluid.addComponent("nC10", 0.30);
    fluid.addTBPfraction("C15", 0.10, 0.150, 0.820);
    fluid.setMixingRule("classic");
    fluid.init(0);
    fluid.init(1);
    return fluid;
  }

  private static void regressionResult(String at, RegressionResult result) {
    for (RegressionParameterConfig config : result.getParameterConfigs()) {
      String name = config.getParameter().name();
      line(at + "value_" + name, config.getOptimizedValue());
      line(at + "lower_" + name, config.getLowerBound());
      line(at + "upper_" + name, config.getUpperBound());
    }
    for (Map.Entry<ExperimentType, Double> objective : result.getObjectiveValues().entrySet()) {
      line(at + "objective_" + objective.getKey().name(), objective.getValue());
    }
    line(at + "final_chi_square", result.getFinalChiSquare());
    line(at + "total_objective", result.getTotalObjective());
    line(at + "average_absolute_deviation_percent", result.getAverageAbsoluteDeviation());
    UncertaintyAnalysis uncertainty = result.getUncertainty();
    line(at + "degrees_of_freedom", uncertainty.getDegreesOfFreedom());
    line(at + "residual_variance", uncertainty.getResidualVariance());
    for (int i = 0; i < uncertainty.getParameterValues().length; i++) {
      line(at + "standard_error_" + i, uncertainty.getStandardError(i));
      line(at + "half_width_95_" + i, uncertainty.getConfidenceInterval95(i));
    }
  }

  // The regression's own optimizer result, from the private sample builders the class's
  // weighting tests also reach by reflection.
  private static void optimizerResult(PVTRegression regression, String builder, int maxIterations)
      throws Exception {
    Method create = PVTRegression.class.getDeclaredMethod("createRegressionFunction");
    create.setAccessible(true);
    PVTRegressionFunction function = (PVTRegressionFunction) create.invoke(regression);
    Method add = PVTRegression.class.getDeclaredMethod(builder, ArrayList.class,
        PVTRegressionFunction.class);
    add.setAccessible(true);
    ArrayList<SampleValue> samples = new ArrayList<>();
    add.invoke(regression, samples, function);
    LevenbergMarquardt optimizer = new LevenbergMarquardt();
    optimizer.setMaxNumberOfIterations(maxIterations);
    optimizer.setSampleSet(new SampleSet(samples));
    optimizer.solve();
    LevenbergMarquardtResult result = optimizer.getResult();
    line("lm_iterations", result.getIterations());
    line("lm_gradient_norm", result.getGradientNorm());
    System.out.println("lm_convergence=" + result.getConvergenceReason().name());
  }

  private static void regression() throws Exception {
    System.out.println("pvt_regression");

    // 1. The class's own CSP viscosity fit: n-heptane, 320 K, 1/5/15 bara, target 1.35.
    System.out.println("[csp_viscosity]");
    double viscosityTemperature = 320.0;
    double[] viscosityPressures = {1.0, 5.0, 15.0};
    double targetFactor = 1.35;
    double[] viscosities = new double[viscosityPressures.length];
    for (int i = 0; i < viscosityPressures.length; i++) {
      viscosities[i] = oilViscosity(viscosityFluid(), viscosityPressures[i], viscosityTemperature,
          targetFactor);
      line("observation_" + i, viscosities[i]);
      line("base_" + i, oilViscosity(viscosityFluid(), viscosityPressures[i], viscosityTemperature,
          1.0));
    }
    PVTRegression viscosity = new PVTRegression(viscosityFluid());
    viscosity.addRegressionParameter(RegressionParameter.VISCOSITY_CSP_3, 0.5, 2.0, 1.0);
    viscosity.addViscosityData(viscosityPressures, viscosities, viscosityTemperature, "oil");
    viscosity.setMaxIterations(8);
    viscosity.setVerbose(false);
    regressionResult("csp_", viscosity.runRegression());
    optimizerResult(viscosity, "addViscositySamples", 8);

    // 2. CCE on a TBP fluid, tuning the C7+ critical-temperature multiplier from a clone at 1.03.
    System.out.println("[cce_tc_multiplier]");
    double[] ccePressures = {250.0, 200.0, 150.0};
    double cceTemperature = 373.15;
    SystemInterface cceBase = tbpFluid();
    componentRows("component_", cceBase);
    SystemInterface cceObserved = SerializationUtils.clone(cceBase);
    cceObserved.init(0);
    cceObserved.init(1);
    RegressionParameter.TC_MULTIPLIER_C7PLUS.applyToFluid(cceObserved, 1.03);
    ConstantMassExpansion cce = new ConstantMassExpansion(cceObserved);
    cce.setTemperature(cceTemperature);
    cce.setPressures(ccePressures);
    cce.runCalc();
    double[] relativeVolumes = cce.getRelativeVolume();
    for (int i = 0; i < ccePressures.length; i++) {
      line("observation_" + i, relativeVolumes[i]);
    }
    ConstantMassExpansion cceBaseState = new ConstantMassExpansion(cceBase.clone());
    cceBaseState.setTemperature(cceTemperature);
    cceBaseState.setPressures(ccePressures);
    cceBaseState.runCalc();
    double[] baseVolumes = cceBaseState.getRelativeVolume();
    for (int i = 0; i < ccePressures.length; i++) {
      line("base_" + i, baseVolumes[i]);
    }
    PVTRegression cceRegression = new PVTRegression(cceBase);
    cceRegression.addCCEData(ccePressures, relativeVolumes, cceTemperature);
    cceRegression.addRegressionParameter(RegressionParameter.TC_MULTIPLIER_C7PLUS, 0.95, 1.05, 1.0);
    cceRegression.setMaxIterations(6);
    cceRegression.setVerbose(false);
    regressionResult("cce_", cceRegression.runRegression());
    optimizerResult(cceRegression, "addCCESamples", 6);

    // 3. DLE tuning the CO2-hydrocarbon BIP from a clone at 0.15.
    System.out.println("[dle_bip_co2]");
    double[] dlePressures = {250.0, 150.0, 80.0};
    double dleTemperature = 323.15;
    SystemInterface dleBase = new SystemSrkEos(dleTemperature, 250.0);
    dleBase.addComponent("methane", 0.45);
    dleBase.addComponent("CO2", 0.05);
    dleBase.addComponent("nC10", 0.50);
    dleBase.setMixingRule("classic");
    dleBase.init(0);
    dleBase.init(1);
    componentRows("component_", dleBase);
    SystemInterface dleObserved = SerializationUtils.clone(dleBase);
    RegressionParameter.BIP_CO2_HC.applyToFluid(dleObserved, 0.15);
    dleObserved.init(0);
    dleObserved.init(1);
    DifferentialLiberation dle = new DifferentialLiberation(dleObserved);
    dle.setTemperature(dleTemperature);
    dle.setPressures(dlePressures);
    dle.runCalc();
    double[] rs = dle.getRs();
    double[] bo = dle.getBo();
    double[] oilDensity = dle.getOilDensity();
    for (int i = 0; i < dlePressures.length; i++) {
      line("observation_rs_" + i, rs[i]);
      line("observation_bo_" + i, bo[i]);
      line("observation_oil_density_" + i, oilDensity[i]);
    }
    DifferentialLiberation dleBaseState = new DifferentialLiberation(dleBase.clone());
    dleBaseState.setTemperature(dleTemperature);
    dleBaseState.setPressures(dlePressures);
    dleBaseState.runCalc();
    double[] baseRs = dleBaseState.getRs();
    double[] baseBo = dleBaseState.getBo();
    for (int i = 0; i < dlePressures.length; i++) {
      line("base_rs_" + i, baseRs[i]);
      line("base_bo_" + i, baseBo[i]);
    }
    PVTRegression dleRegression = new PVTRegression(dleBase);
    dleRegression.addDLEData(dlePressures, rs, bo, oilDensity, dleTemperature);
    dleRegression.addRegressionParameter(RegressionParameter.BIP_CO2_HC, 0.08, 0.18, 0.12);
    dleRegression.setMaxIterations(6);
    dleRegression.setVerbose(false);
    regressionResult("dle_", dleRegression.runRegression());
    optimizerResult(dleRegression, "addDLESamples", 6);
  }

  private static SystemInterface tuningFluid(double temperature) {
    SystemInterface fluid = new SystemSrkEos(temperature, 100.0);
    fluid.addComponent("methane", 0.80);
    fluid.addComponent("nC10", 0.10);
    fluid.addPlusFraction("C11", 0.10, 0.2562, 0.8398);
    fluid.setMixingRule("classic");
    fluid.init(0);
    fluid.init(1);
    return fluid;
  }

  private static void tuningBlock(String block, double temperature, double offset) {
    System.out.println("[" + block + "]");
    SystemInterface base = tuningFluid(temperature);
    componentRows("component_", base);
    SystemInterface characterised = SerializationUtils.clone(base);
    characterised.getCharacterization().characterisePlusFraction();
    characterised.createDatabase(true);
    characterised.setMixingRule("classic");
    characterised.init(0);
    characterised.init(1);
    SaturationPressure baseScan = new SaturationPressure(characterised);
    baseScan.setTemperature(temperature);
    double baseSaturation = baseScan.calcSaturationPressure();
    line("base_saturation_pressure_pa", baseSaturation * 1.0e5);
    double targetBar = baseSaturation + offset;
    line("target_saturation_pressure_pa", targetBar * 1.0e5);

    SaturationPressure simulation = new SaturationPressure(base);
    TuneToSaturation tuner = new TuneToSaturation(simulation);
    tuner.setSaturationConditions(temperature, targetBar);
    java.io.ByteArrayOutputStream captured = new java.io.ByteArrayOutputStream();
    java.io.PrintStream previous = System.out;
    try {
      System.setOut(new java.io.PrintStream(captured, true));
      tuner.run();
    } finally {
      System.setOut(previous);
    }
    int iterations = 0;
    double tunedMolarMass = 0.2562;
    for (String row : captured.toString().split("\\R")) {
      int at = row.indexOf("molarmass");
      if (at >= 0) {
        iterations++;
        try {
          tunedMolarMass = Double.parseDouble(row.substring(at + "molarmass".length()).trim());
        } catch (NumberFormatException ignored) {
          // A line whose number did not parse leaves the previous value.
        }
      }
    }
    line("initial_molar_mass", 0.2562);
    line("tuned_molar_mass", tunedMolarMass);
    line("iterations", iterations);
    line("final_saturation_pressure_pa", simulation.getSaturationPressure() * 1.0e5);
  }

  private static void tuning() {
    System.out.println("pvt_tune_to_saturation");
    // The first run's error is under the class's 0.01-bar early return: no update, no loop.
    tuningBlock("early_return", 350.0, 0.003);
    // A 0.02-bar target: the sign-flipping loop runs and exits on its lagged criterion.
    tuningBlock("loop", 350.0, 0.02);
  }

  public static void main(String[] args) throws Exception {
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
      case "gas_mb":
        gasMaterialBalance();
        break;
      case "oil_mb":
        oilMaterialBalance();
        break;
      case "saturation":
        saturation();
        break;
      case "cce":
        cce();
        break;
      case "dle":
        dle();
        break;
      case "cvd":
        cvd();
        break;
      case "separator":
        separator();
        break;
      case "swelling":
        swelling();
        break;
      case "mmp":
        mmp();
        break;
      case "property_sweep":
        propertySweep();
        break;
      case "regression":
        regression();
        break;
      case "tuning":
        tuning();
        break;
      default:
        System.err.println("unknown subcommand: " + which);
        System.exit(2);
    }
  }
}
