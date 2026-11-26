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
import neqsim.blackoil.BlackOilPVTTable.Record;
import neqsim.pvtsimulation.modeltuning.TuneToSaturation;
import neqsim.pvtsimulation.regression.ExperimentType;
import neqsim.pvtsimulation.regression.PVTRegression;
import neqsim.pvtsimulation.regression.PVTRegressionFunction;
import neqsim.pvtsimulation.regression.RegressionParameter;
import neqsim.pvtsimulation.regression.RegressionParameterConfig;
import neqsim.pvtsimulation.regression.RegressionResult;
import neqsim.pvtsimulation.regression.UncertaintyAnalysis;
import neqsim.pvtsimulation.reservoirproperties.CompositionEstimation;
import neqsim.pvtsimulation.reservoirproperties.materialbalance.FetkovichAquifer;
import neqsim.pvtsimulation.reservoirproperties.materialbalance.GasMaterialBalance;
import neqsim.pvtsimulation.reservoirproperties.materialbalance.GeneralizedMaterialBalance;
import neqsim.pvtsimulation.reservoirproperties.materialbalance.OilMaterialBalance;
import neqsim.pvtsimulation.reservoirproperties.materialbalance.VanEverdingenHurstAquifer;
import neqsim.pvtsimulation.reservoirproperties.relpermeability.RelPermModelFamily;
import neqsim.pvtsimulation.reservoirproperties.relpermeability.RelPermTableType;
import neqsim.pvtsimulation.reservoirproperties.relpermeability.RelativePermeabilityGenerator;
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
import neqsim.pvtsimulation.util.CvdMaterialBalance;
import neqsim.pvtsimulation.util.DeclineCurveAnalysis;
import neqsim.pvtsimulation.util.BlackOilUnits;
import neqsim.pvtsimulation.util.GasPseudoCriticalProperties;
import neqsim.pvtsimulation.util.GasPseudoPressure;
import neqsim.pvtsimulation.util.PvtGasDensityConsistency;
import neqsim.pvtsimulation.util.PvtSeparatorPath;
import neqsim.pvtsimulation.util.SaturationPressureCorrelation;
import neqsim.pvtsimulation.util.WaterPropertyCorrelations;
import neqsim.pvtsimulation.util.ZFactorCorrelations;
import neqsim.statistics.parameterfitting.SampleSet;
import neqsim.statistics.parameterfitting.SampleValue;
import neqsim.statistics.parameterfitting.nonlinearparameterfitting.LevenbergMarquardt;
import neqsim.statistics.parameterfitting.nonlinearparameterfitting.LevenbergMarquardtResult;
import neqsim.thermo.component.ComponentInterface;
import neqsim.thermo.system.SystemInterface;
import neqsim.thermo.system.SystemPrEos;
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

  // The CVD accounting state: the class's analytic binary and ternary paths, the published
  // NS-1 Table 1 (11 components, seven rows), and the liquid-state limits. No EOS and no
  // flash; the probe supplies the observations and prints the reconstructed inventories.
  private static double sum(double[] values) {
    double total = 0.0;
    for (double value : values) total += value;
    return total;
  }

  private static void cvdAccounting(String block, String[] names, double[] pressuresPa,
      double[] gasZ, double[] liquidVolumes, double[] cumulative, double[][] vapor,
      double balanceTolerance, double minimumLiquidMoles) {
    System.out.println("[" + block + "]");
    System.out.println("components=" + names.length);
    for (int j = 0; j < names.length; j++) {
      System.out.println("component_" + j + "_name=" + names[j]);
    }
    CvdMaterialBalance.Observation[] rows = new CvdMaterialBalance.Observation[pressuresPa.length];
    for (int k = 0; k < rows.length; k++) {
      rows[k] = new CvdMaterialBalance.Observation("row-" + k, pressuresPa[k], gasZ[k],
          liquidVolumes[k], cumulative[k], vapor[k]);
    }
    CvdMaterialBalance report = new CvdMaterialBalance("synthetic", "probe", "probe", 350.0,
        names, rows, balanceTolerance, minimumLiquidMoles, null);
    line("observation_count", report.getObservationCount());
    line("compatible", report.isCompatible() ? 1.0 : 0.0);
    for (int k = 0; k < rows.length; k++) {
      CvdMaterialBalance.Stage stage = report.getStage(k);
      String at = "stage_" + k + "_";
      line(at + "total_moles", stage.getTotalRemainingMoles());
      line(at + "vapor_moles", stage.getVaporMoles());
      line(at + "liquid_moles", stage.getLiquidMoles());
      line(at + "stage_withdrawn_moles", stage.getStageWithdrawnMoles());
      line(at + "component_closure", sum(stage.getLiquidComponentMoles()) - stage.getLiquidMoles());
      line(at + "remaining_closure", sum(stage.getRemainingComponentMoles())
          - stage.getTotalRemainingMoles());
      line(at + "produced_closure", sum(stage.getProducedComponentMoles()) - cumulative[k]);
      line(at + "vapor_closure", stage.getVaporMoles() * sum(vapor[k]) - stage.getVaporMoles());
      System.out.println(at + "liquid_state=" + stage.getLiquidState().name().toLowerCase(java.util.Locale.ROOT));
      System.out.println(at + "compatible=" + ((Boolean) field(stage, "compatible") ? 1 : 0));
      double[] produced = stage.getProducedComponentMoles();
      double[] remaining = stage.getRemainingComponentMoles();
      double[] liquid = stage.getLiquidComponentMoles();
      for (int j = 0; j < names.length; j++) {
        line(at + "component_" + j + "_produced_moles", produced[j]);
        line(at + "component_" + j + "_remaining_moles", remaining[j]);
        line(at + "component_" + j + "_liquid_moles", liquid[j]);
      }
    }
  }

  private static void cvdMaterialBalance() {
    System.out.println("cvd_material_balance");
    cvdAccounting("analytic_binary", new String[] {"A", "B"},
        new double[] {100.0e5, 50.0e5, 25.0e5}, new double[] {1.0, 1.0, 1.0},
        new double[] {0.0, 0.2, 0.2}, new double[] {0.0, 0.2, 0.4},
        new double[][] {{0.8, 0.2}, {0.9, 0.1}, {0.9, 0.1}}, 1.0e-12, 1.0e-8);

    cvdAccounting("analytic_ternary", new String[] {"component-0", "component-1", "component-2"},
        new double[] {100.0e5, 100.0e5 * 5.0 / 9.0, 100.0e5 * 3.0 / 8.0},
        new double[] {1.0, 1.0, 1.0}, new double[] {0.0, 0.1, 0.2},
        new double[] {0.0, 0.2, 0.3},
        new double[][] {{0.48, 0.33, 0.19}, {0.6, 0.3, 0.1}, {0.65, 0.3, 0.05}}, 1.0e-12, 1.0e-8);

    // Whitson-Torp NS-1 Table 1: psia, Z, cumulative percent, liquid volume percent, and
    // eleven vapor analyses in percent.
    String[] ns1Names = {"CO2", "N2", "C1", "C2", "C3", "iC4", "nC4", "iC5", "nC5", "C6", "C7+"};
    double[][] ns1 = {
        {6764.7, 1.238, 0.000, 0.0, 2.37, 0.31, 73.19, 7.80, 3.55, 0.71, 1.45, 0.64, 0.68, 1.09, 8.21},
        {5514.7, 1.089, 9.024, 14.1, 2.40, 0.32, 75.56, 7.83, 3.47, 0.67, 1.37, 0.59, 0.62, 0.97, 6.20},
        {4314.7, 0.972, 21.744, 19.7, 2.45, 0.33, 77.89, 7.87, 3.40, 0.65, 1.31, 0.55, 0.58, 0.88, 4.09},
        {3114.7, 0.913, 38.674, 21.6, 2.50, 0.34, 79.33, 7.92, 3.41, 0.64, 1.30, 0.53, 0.56, 0.83, 2.64},
        {2114.7, 0.914, 55.686, 21.3, 2.53, 0.34, 79.62, 8.04, 3.53, 0.66, 1.33, 0.54, 0.57, 0.82, 2.02},
        {1214.7, 0.937, 72.146, 20.2, 2.57, 0.34, 78.90, 8.40, 3.74, 0.72, 1.44, 0.59, 0.61, 0.85, 1.84},
        {714.7, 0.960, 81.301, 19.3, 2.60, 0.33, 77.80, 8.70, 3.91, 0.78, 1.56, 0.64, 0.66, 0.90, 2.12}};
    double[] ns1Pressure = new double[ns1.length];
    double[] ns1Z = new double[ns1.length];
    double[] ns1Cumulative = new double[ns1.length];
    double[] ns1Liquid = new double[ns1.length];
    double[][] ns1Vapor = new double[ns1.length][ns1Names.length];
    for (int k = 0; k < ns1.length; k++) {
      ns1Pressure[k] = ns1[k][0] * 6894.757293168;
      ns1Z[k] = ns1[k][1];
      ns1Cumulative[k] = ns1[k][2] / 100.0;
      ns1Liquid[k] = ns1[k][3] / 100.0;
      for (int j = 0; j < ns1Names.length; j++) ns1Vapor[k][j] = ns1[k][j + 4] / 100.0;
    }
    cvdAccounting("ns1_table1", ns1Names, ns1Pressure, ns1Z, ns1Liquid, ns1Cumulative, ns1Vapor,
        1.0e-12, 1.0e-8);

    cvdAccounting("gas_only", new String[] {"A"}, new double[] {100.0, 50.0},
        new double[] {1.0, 1.0}, new double[] {0.0, 0.0}, new double[] {0.0, 0.5},
        new double[][] {{1.0}, {1.0}}, 1.0e-12, 1.0e-8);

    cvdAccounting("vanishing_liquid", new String[] {"A"}, new double[] {100.0, 50.0},
        new double[] {1.0, 1.0}, new double[] {0.0, 0.2}, new double[] {0.0, 0.6 - 1.0e-10},
        new double[][] {{1.0}, {1.0}}, 1.0e-12, 1.0e-8);

    cvdAccounting("negative_component", new String[] {"A", "B"},
        new double[] {100.0e5, 50.0e5}, new double[] {1.0, 1.0}, new double[] {0.0, 0.2},
        new double[] {0.0, 0.2}, new double[][] {{0.2, 0.8}, {0.9, 0.1}}, 1.0e-12, 1.0e-8);
  }

  // The gas-density identity: rho = p M / (Z R T) with outward-rounded interval endpoints.
  private static double[] densityRow(String id, double p, double pBound, double t, double tBound,
      double z, double zBound, double m, double mBound, double density, double densityBound) {
    PvtGasDensityConsistency.Observation row = new PvtGasDensityConsistency.Observation(id, "probe",
        "probe", "probe", new PvtGasDensityConsistency.Quantity(p, pBound),
        new PvtGasDensityConsistency.Quantity(t, tBound), new PvtGasDensityConsistency.Quantity(z, zBound),
        new PvtGasDensityConsistency.Quantity(m, mBound),
        new PvtGasDensityConsistency.Quantity(density, densityBound));
    return new double[] {row.getImpliedDensityKgPerM3(), row.getImpliedLowerKgPerM3(),
        row.getImpliedUpperKgPerM3(), density - row.getImpliedDensityKgPerM3(),
        row.isCompatible() ? 1.0 : 0.0};
  }

  private static void densityRows(String block, double[][] rows, String[] ids) {
    System.out.println("[" + block + "]");
    for (int k = 0; k < ids.length; k++) {
      try {
        double[] row = densityRow(ids[k], rows[k][0], rows[k][1], rows[k][2], rows[k][3],
            rows[k][4], rows[k][5], rows[k][6], rows[k][7], rows[k][8], rows[k][9]);
        String at = "row_" + k + "_";
        line(at + "implied_density", row[0]);
        line(at + "implied_lower", row[1]);
        line(at + "implied_upper", row[2]);
        line(at + "observed_minus_implied", row[3]);
        line(at + "compatible", row[4]);
      } catch (RuntimeException error) {
        System.out.println("row_" + k + "_refused=" + error.getMessage().replace('\n', ' '));
      }
    }
  }

  private static void gasDensityConsistency() {
    System.out.println("gas_density_consistency");
    double psi = 6894.757293168;
    String[] duvernayIds = {"Psat", "Step1", "Step2", "Step3", "Step4", "Step5", "Step6",
        "Step7", "Step8"};
    double[][] duvernay = {
        {4661 * psi, 0.5 * psi, 379.15, 0.5, 0.922, 0.0005, 0.03018, 0.000005, 339.0, 0.5},
        {4451 * psi, 0.5 * psi, 379.15, 0.5, 0.897, 0.0005, 0.02987, 0.000005, 324.0, 0.5},
        {4100 * psi, 0.5 * psi, 379.15, 0.5, 0.861, 0.0005, 0.02893, 0.000005, 302.0, 0.5},
        {3701 * psi, 0.5 * psi, 379.15, 0.5, 0.828, 0.0005, 0.02755, 0.000005, 270.0, 0.5},
        {3201 * psi, 0.5 * psi, 379.15, 0.5, 0.813, 0.0005, 0.02595, 0.000005, 224.0, 0.5},
        {2600 * psi, 0.5 * psi, 379.15, 0.5, 0.813, 0.0005, 0.02456, 0.000005, 172.0, 0.5},
        {2100 * psi, 0.5 * psi, 379.15, 0.5, 0.826, 0.0005, 0.02384, 0.000005, 133.0, 0.5},
        {1650 * psi, 0.5 * psi, 379.15, 0.5, 0.848, 0.0005, 0.02311, 0.000005, 98.0, 0.5},
        {1300 * psi, 0.5 * psi, 379.15, 0.5, 0.874, 0.0005, 0.02314, 0.000005, 75.0, 0.5}};
    densityRows("duvernay_table18", duvernay, duvernayIds);

    String[] analyticIds = {"point_20", "touching_21_1", "gap_21_099", "dilute", "corners"};
    double[][] analytic = {
        {831446.261815324, 0.0, 100.0, 0.0, 1.0, 0.0, 0.02, 0.0, 20.0, 0.0},
        {831446.261815324, 0.0, 100.0, 0.0, 1.0, 0.0, 0.02, 0.0, 21.0, 1.0},
        {831446.261815324, 0.0, 100.0, 0.0, 1.0, 0.0, 0.02, 0.0, 21.0, 0.99},
        {1.0e-6, 0.0, 100.0, 0.0, 1.0, 0.0, 0.02, 0.0, 2.4054471008545208e-11, 1.0e-25},
        {20.0e6, 1.0e5, 350.0, 0.5, 0.9, 0.01, 0.025, 0.0001, 200.0, 100.0}};
    densityRows("analytic", analytic, analyticIds);
  }

  // The separator path: the class's own four states, on the PR mixture its test builds.
  private static SystemInterface pathSample() {
    SystemInterface fluid = new SystemPrEos(350.0, 100.0);
    fluid.addComponent("methane", 70.0);
    fluid.addComponent("ethane", 10.0);
    fluid.addComponent("propane", 5.0);
    fluid.addComponent("n-decane", 15.0);
    fluid.setMixingRule("classic");
    return fluid;
  }

  private static void separatorPath(String block, SystemInterface feed, double[] pressuresBara,
      double[] temperatures) {
    System.out.println("[" + block + "]");
    PvtSeparatorPath.Result result = new PvtSeparatorPath(pressuresBara, temperatures, "probe")
        .separate(feed);
    line("final_pressure_pa", result.getFinalPressureBara() * 1.0e5);
    line("final_temperature_k", result.getFinalTemperatureK());
    double[] volumes = result.getVolumesM3();
    line("gas_volume_m3", volumes[0]);
    line("oil_volume_m3", volumes[1]);
    line("aqueous_volume_m3", volumes[2]);
    if (volumes[1] > 0.0) {
      line("gas_oil_ratio", result.getGasOilRatio());
    } else {
      System.out.println("gas_oil_ratio=absent");
    }
    String[] names = result.getComponentNames();
    double[][] components = result.getComponentMoles();
    for (int p = 0; p < 3; p++) {
      for (int j = 0; j < names.length; j++) {
        line("phase_" + p + "_" + names[j] + "_moles", components[p][j]);
      }
    }
  }

  private static void separatorPath() {
    System.out.println("separator_path");
    separatorPath("one_stage_stock_tank", pathSample(), new double[] {1.01325},
        new double[] {288.15});
    separatorPath("two_stage_offgas", pathSample(), new double[] {20.0, 1.01325},
        new double[] {400.0, 288.15});
    SystemInterface threePhase = pathSample();
    threePhase.addComponent("water", 10.0);
    threePhase.setMixingRule("classic");
    threePhase.setMultiPhaseCheck(true);
    separatorPath("three_phase", threePhase, new double[] {10.0, 1.01325},
        new double[] {310.0, 288.15});
    SystemInterface gas = new SystemPrEos(300.0, 10.0);
    gas.addComponent("methane", 100.0);
    gas.setMixingRule("classic");
    separatorPath("gas_only", gas, new double[] {1.01325}, new double[] {288.15});
    System.out.println("[declarations]");
    refusal("refused_increasing_pressure", () -> new PvtSeparatorPath(
        new double[] {10.0, 20.0}, new double[] {300.0, 288.15}, "invalid"));
    refusal("refused_nonfinite_temperature", () -> new PvtSeparatorPath(
        new double[] {1.0}, new double[] {Double.NaN}, "invalid"));
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

  /// Prints whether a call was refused, and why, instead of aborting the whole capture.
  private static void refusal(String label, Runnable call) {
    try {
      call.run();
      System.out.println(label + "=accepted");
    } catch (RuntimeException error) {
      System.out.println(label + "=" + error.getMessage().replace('\n', ' '));
    }
  }

  /// The Van Everdingen-Hurst aquifer: the kernel's states, the time and constant conversions,
  /// the Carter-Tracy recurrence and the influence table. The AQUTAB text is a report and is
  /// printed only as the pressure values it contains.
  private static void aquifer() {
    final double INF = Double.POSITIVE_INFINITY;
    System.out.println("pvt_van_everdingen_hurst");

    // The test's infinite-acting monotonic points and its late-time semilog check.
    double[] tDs = {0.005, 0.05, 0.5, 5.0, 50.0, 150.0, 500.0, 5000.0};
    for (int i = 0; i < tDs.length; i++) {
      line("pd_" + i, VanEverdingenHurstAquifer.dimensionlessPressure(tDs[i], INF));
      line("dpd_" + i, VanEverdingenHurstAquifer.dimensionlessPressureDerivative(tDs[i], INF));
    }
    line("pd_late_semilog", VanEverdingenHurstAquifer.dimensionlessPressure(1.0e5, INF));
    line("pd_bounded_100_5", VanEverdingenHurstAquifer.dimensionlessPressure(100.0, 5.0));
    line("dpd_bounded_1e-4_10",
        VanEverdingenHurstAquifer.dimensionlessPressureDerivative(1.0e-4, 10.0));

    // The independent-reference times; the class's values at them are the oracle.
    double[] referenceTimes = {1e-12, 1e-8, 1e-6, 3e-5, 1e-4, 0.003, 0.009999, 0.01, 0.010001, 0.3,
        1.0, 7.5, 100.0, 199.999, 200.0, 200.001, 8000.0, 10000.0, 1e8, 1e12};
    for (int i = 0; i < referenceTimes.length; i++) {
      line("ref_td_" + i, referenceTimes[i]);
      line("ref_pd_" + i, VanEverdingenHurstAquifer.dimensionlessPressure(referenceTimes[i], INF));
      line("ref_dpd_" + i,
          VanEverdingenHurstAquifer.dimensionlessPressureDerivative(referenceTimes[i], INF));
    }

    line("dimensionless_time_test",
        VanEverdingenHurstAquifer.dimensionlessTime(1.0e-13, 8.64e7, 0.2, 5.0e-4, 1.0e-9, 3000.0));
    line("dimensionless_time_zero_k",
        VanEverdingenHurstAquifer.dimensionlessTime(0.0, 8.64e7, 0.2, 5.0e-4, 1.0e-9, 3000.0));
    line("dimensionless_time_zero_t",
        VanEverdingenHurstAquifer.dimensionlessTime(1.0e-13, 0.0, 0.2, 5.0e-4, 1.0e-9, 3000.0));

    double uBar = VanEverdingenHurstAquifer.aquiferConstant(0.20, 1.0e-4, 30.0, 3000.0, 180.0);
    line("aquifer_constant_bar", uBar);
    line("aquifer_constant_pa", uBar * 1.0e-5);
    line("aquifer_constant_zero_porosity",
        VanEverdingenHurstAquifer.aquiferConstant(0.0, 1.0e-4, 30.0, 3000.0, 180.0));
    line("aquifer_constant_zero_angle",
        VanEverdingenHurstAquifer.aquiferConstant(0.20, 1.0e-4, 30.0, 3000.0, 0.0));

    double[] ctTimes = {0.0, 10.0, 20.0, 40.0, 80.0, 160.0};
    double[] ctDropsBar = {0.0, 5.0, 12.0, 22.0, 35.0, 50.0};
    double[] we = VanEverdingenHurstAquifer.cumulativeInfluxCarterTracy(ctTimes, ctDropsBar, uBar, INF);
    for (int i = 0; i < ctTimes.length; i++) {
      line("carter_tracy_td_" + i, ctTimes[i]);
      line("carter_tracy_drop_pa_" + i, ctDropsBar[i] * 1.0e5);
      line("carter_tracy_we_" + i, we[i]);
    }
    double[] zeroSupport =
        VanEverdingenHurstAquifer.cumulativeInfluxCarterTracy(ctTimes, ctDropsBar, 0.0, INF);
    for (int i = 0; i < zeroSupport.length; i++) {
      line("carter_tracy_zero_we_" + i, zeroSupport[i]);
    }

    double[] recoveryTimes = {0.0, 0.1, 0.2};
    double[] recoveryDrops = {0.0, 1.0, -10.0};
    double[] recovered =
        VanEverdingenHurstAquifer.cumulativeInfluxCarterTracy(recoveryTimes, recoveryDrops, 1.0, INF);
    for (int i = 0; i < recovered.length; i++) {
      line("carter_tracy_recovery_we_" + i, recovered[i]);
    }

    // The first-step identity t/P_D, and the scale-loss rows the recurrence's products survive.
    double[] extremeTimes = {Double.MIN_VALUE, 1e-300, 1e-200, 1e-100, 1e-70, 1e-50, 1e-8, 1.0};
    for (int i = 0; i < extremeTimes.length; i++) {
      double first = VanEverdingenHurstAquifer.cumulativeInfluxCarterTracy(
          new double[] {0.0, extremeTimes[i]}, new double[] {0.0, 1.0}, 1.0, INF)[1];
      line("first_step_we_" + i, first);
      line("first_step_expected_" + i,
          extremeTimes[i] / VanEverdingenHurstAquifer.dimensionlessPressure(extremeTimes[i], INF));
    }
    double[][] scaleRows = {{1e-300, 1e200, 1e200}, {Double.MAX_VALUE, 1e-200, 1e-200},
        {1e-200, 1e250, 1e100}, {1e100, 1e-250, 1e-100}};
    for (int i = 0; i < scaleRows.length; i++) {
      line("scale_we_" + i, VanEverdingenHurstAquifer.cumulativeInfluxCarterTracy(
          new double[] {0.0, scaleRows[i][0]}, new double[] {0.0, scaleRows[i][2]}, scaleRows[i][1],
          INF)[1]);
    }
    line("bounded_first_step",
        VanEverdingenHurstAquifer.cumulativeInfluxCarterTracy(new double[] {0.0, Double.MIN_VALUE},
            new double[] {0.0, 1e200}, 1e200, 100.0)[1]);

    double[] overflowRecovered = VanEverdingenHurstAquifer.cumulativeInfluxCarterTracy(
        new double[] {0.0, 1e-300, 1e100}, new double[] {0.0, 1e200, -1e200}, 1e200, INF);
    line("recovered_we_1", overflowRecovered[1]);
    line("recovered_we_2", overflowRecovered[2]);
    refusal("refused_later_overflow",
        () -> VanEverdingenHurstAquifer.cumulativeInfluxCarterTracy(new double[] {0.0, 1e-200, 1e200},
            new double[] {0.0, 1e100, 1e100}, 1e100, INF));
    refusal("refused_unrepresentable",
        () -> VanEverdingenHurstAquifer.cumulativeInfluxCarterTracy(new double[] {0.0, 1e100},
            new double[] {0.0, Double.MAX_VALUE}, Double.MAX_VALUE, INF));
    refusal("refused_empty",
        () -> VanEverdingenHurstAquifer.cumulativeInfluxCarterTracy(new double[0], new double[0], 1.0,
            INF));
    refusal("refused_non_increasing", () -> VanEverdingenHurstAquifer
        .cumulativeInfluxCarterTracy(new double[] {0.0, 0.0}, new double[] {0.0, 1.0}, 1.0, INF));
    refusal("refused_negative_time",
        () -> VanEverdingenHurstAquifer.dimensionlessPressure(-1.0, INF));
    refusal("refused_nan_radius",
        () -> VanEverdingenHurstAquifer.dimensionlessPressure(1.0, Double.NaN));

    double[] tableTimes = {1.0, 10.0, 100.0, 1000.0};
    double[] tablePressure = VanEverdingenHurstAquifer.influenceTable(tableTimes, INF);
    for (int i = 0; i < tableTimes.length; i++) {
      line("influence_table_td_" + i, tableTimes[i]);
      line("influence_table_pd_" + i, tablePressure[i]);
    }
  }

  /// Prints one Fetkovich state in the id's SI units.
  private static void fetkovichState(String label, FetkovichAquifer.State state) {
    line(label + "_time_s", state.getTimeSeconds());
    line(label + "_cumulative_influx_m3", state.getCumulativeInfluxM3());
    line(label + "_pressure_pa", state.getPressureBara() * 1.0e5);
    line(label + "_interval_influx_m3", state.getIntervalInfluxM3());
    line(label + "_minimum_influx_m3", state.getIntervalMinimumCumulativeInfluxM3());
    line(label + "_maximum_influx_m3", state.getIntervalMaximumCumulativeInfluxM3());
  }

  /// The Fetkovich finite aquifer: every branch the class's own tests pin, plus the refusal rows.
  private static void fetkovich() {
    System.out.println("pvt_fetkovich_aquifer");

    FetkovichAquifer constant = new FetkovichAquifer(1.0e6, 1.0e-4, 0.02, 300.0);
    fetkovichState("constant_pressure",
        constant.advance(constant.initialState(), 12000.0, 250.0, 250.0));
    FetkovichAquifer ramp = new FetkovichAquifer(1.0e6, 1.0e-4, 0.02, 300.0);
    fetkovichState("linear_ramp", ramp.advance(ramp.initialState(), 20000.0, 300.0, 180.0));

    FetkovichAquifer small = new FetkovichAquifer(1.0e12, 1.0e-4, 0.02, 300.0);
    fetkovichState("tiny_exposure", small.advance(small.initialState(), 1.0e-5, 300.0, 250.0));
    fetkovichState("tiny_extrema", small.advance(small.initialState(), 1.0e-5, 250.0, 400.0));
    fetkovichState("large_step", small.advance(small.initialState(), 1.0e20, 300.0, 200.0));

    FetkovichAquifer finite = new FetkovichAquifer(1.0e6, 1.0e-4, 0.02, 300.0);
    fetkovichState("ramp_up_extrema", finite.advance(finite.initialState(), 10000.0, 250.0, 400.0));
    fetkovichState("ramp_down_extrema",
        finite.advance(finite.initialState(), 10000.0, 350.0, 200.0));

    FetkovichAquifer infinite = new FetkovichAquifer(Double.POSITIVE_INFINITY, 1.0e-4, 0.02, 300.0);
    line("infinite_capacity", infinite.getCapacityM3PerBar());
    fetkovichState("infinite_up", infinite.advance(infinite.initialState(), 1000.0, 250.0, 400.0));
    fetkovichState("infinite_down",
        infinite.advance(infinite.initialState(), 1000.0, 350.0, 200.0));
    fetkovichState("infinite_drawdown",
        infinite.advance(infinite.initialState(), 1000.0, 300.0, 100.0));

    FetkovichAquifer noFlow = new FetkovichAquifer(1.0e6, 1.0e-4, 0.0, 300.0);
    fetkovichState("zero_productivity", noFlow.advance(noFlow.initialState(), 1000.0, 300.0, 100.0));
    FetkovichAquifer rigid = new FetkovichAquifer(1.0e6, 0.0, 0.02, 300.0);
    fetkovichState("zero_storage", rigid.advance(rigid.initialState(), 1000.0, 300.0, 100.0));

    FetkovichAquifer charged = new FetkovichAquifer(1.0e6, 1.0e-4, 0.02, 300.0);
    FetkovichAquifer.State first = charged.advance(charged.initialState(), 100000.0, 350.0, 350.0);
    fetkovichState("recharge", first);
    fetkovichState("recharge_return", charged.advance(first, 100000.0, 300.0, 300.0));

    // The 1e-3 small-exposure series switch, bracketed on both sides and both ramp directions.
    double[] boundaryDurations = {0.00999999, 0.01, 0.01000001};
    double[][] boundaryRamps = {{300.0, 150.0}, {200.0, 450.0}, {400.0, 150.0}};
    for (int i = 0; i < boundaryDurations.length; i++) {
      FetkovichAquifer boundary = new FetkovichAquifer(1000.0, 1.0e-6, 1.0e-5, 300.0);
      for (int j = 0; j < boundaryRamps.length; j++) {
        fetkovichState("boundary_" + i + "_" + j, boundary.advance(boundary.initialState(),
            boundaryDurations[i], boundaryRamps[j][0], boundaryRamps[j][1]));
      }
    }
    double[] laterDurations = {0.01, 1.0, 10.0};
    for (int i = 0; i < laterDurations.length; i++) {
      FetkovichAquifer later = new FetkovichAquifer(1000.0, 1.0e-6, 1.0e-5, 300.0);
      fetkovichState("later_" + i,
          later.advance(later.initialState(), laterDurations[i], 300.0, 150.0));
    }
    FetkovichAquifer strong = new FetkovichAquifer(1.0e6, 1.0e-4, 2.0, 300.0);
    fetkovichState("strong_ramp", strong.advance(strong.initialState(), 0.004999995, 300.0, 150.0));

    refusal("refused_negative_volume", () -> new FetkovichAquifer(-1.0, 1.0e-4, 0.02, 300.0));
    refusal("refused_nan_compressibility", () -> new FetkovichAquifer(1.0, Double.NaN, 0.02, 300.0));
    refusal("refused_negative_productivity", () -> new FetkovichAquifer(1.0, 1.0e-4, -0.02, 300.0));
    refusal("refused_zero_pressure", () -> new FetkovichAquifer(1.0, 1.0e-4, 0.02, 0.0));
    refusal("refused_unrepresentable_capacity",
        () -> new FetkovichAquifer(Double.MAX_VALUE, 2.0, 0.02, 300.0));
    FetkovichAquifer exhausted = new FetkovichAquifer(1.0, 0.1, 1.0, 300.0);
    refusal("refused_zero_duration", () -> exhausted.advance(exhausted.initialState(), 0.0, 300.0, 250.0));
    refusal("refused_nan_pressure",
        () -> exhausted.advance(exhausted.initialState(), 1.0, 300.0, Double.NaN));
    refusal("refused_exhausted_endpoint",
        () -> exhausted.advance(exhausted.initialState(), 100.0, 250.0, 250.0));
    refusal("refused_exhausted_interior",
        () -> exhausted.advance(exhausted.initialState(), 10.0, 250.0, 1000.0));
  }

  /// A PVT record for the generalized balance; the viscosities are unused and carry no invented value.
  private static Record pvtRecord(double pressure, double rs, double bo, double bg, double rv) {
    return new Record(pressure, rs, bo, Double.NaN, bg, Double.NaN, rv, 1.02, Double.NaN);
  }

  /// The class test's synthetic history: independent inventories and a volume closure.
  private static GeneralizedMaterialBalance history(double oil, double gas, boolean support) {
    Record initial = pvtRecord(300.0, 100.0, 1.6, 0.004, 0.0002);
    GeneralizedMaterialBalance balance = new GeneralizedMaterialBalance(initial);
    Record[] records = {pvtRecord(260.0, 80.0, 1.5, 0.005, 0.00016),
        pvtRecord(210.0, 60.0, 1.4, 0.008, 0.00012), pvtRecord(160.0, 40.0, 1.3, 0.012, 0.00008),
        pvtRecord(110.0, 20.0, 1.2, 0.020, 0.00004)};
    double initialVolume = oil * initial.Bo + gas * initial.Bg;
    double initialOil = oil + initial.Rv * gas;
    double initialGas = gas + initial.Rs * oil;
    for (int i = 0; i < records.length; i++) {
      Record current = records[i];
      double fraction = support ? 0.001 * (i + 1) : 0.0;
      double influx = support ? 0.002 * initialVolume * (i + 1) : 0.0;
      double netWater = support ? -0.001 * initialVolume * (i + 1) : 0.0;
      double remainingVolume = initialVolume * (1.0 - fraction) - influx + netWater * current.Bw;
      double oilVolume = remainingVolume * (oil == 0.0 ? 0.01 * (i + 1) : 0.58 - 0.08 * i);
      double gasVolume = remainingVolume - oilVolume;
      double freeOil = oilVolume / current.Bo;
      double freeGas = gasVolume / current.Bg;
      double netOil = initialOil - (freeOil + current.Rv * freeGas);
      double netGas = initialGas - (freeGas + current.Rs * freeOil);
      balance.addObservation(current, netOil, netGas, netWater, influx, fraction);
    }
    return balance;
  }

  /// Prints one fit of the generalized balance.
  private static void materialBalanceFit(String label, GeneralizedMaterialBalance balance,
      String fit, double gasCap) {
    GeneralizedMaterialBalance.Result result;
    switch (fit) {
      case "initially_oil":
        result = balance.fitInitiallyOil();
        break;
      case "initially_gas":
        result = balance.fitInitiallyGas();
        break;
      case "known_gas_cap":
        result = balance.fitKnownGasCap(gasCap);
        break;
      default:
        result = balance.fitOilAndGas();
        break;
    }
    line(label + "_free_oil_m3", result.getInitialFreeOil());
    line(label + "_free_gas_m3", result.getInitialFreeGas());
    line(label + "_ooip_m3", result.getOoip());
    line(label + "_ogip_m3", result.getOgip());
    double ratio = result.getGasCapRatio();
    if (Double.isInfinite(ratio)) {
      System.out.println(label + "_gas_cap_ratio=infinite");
    } else {
      line(label + "_gas_cap_ratio", ratio);
    }
    double[] residuals = result.getResiduals();
    for (int i = 0; i < residuals.length; i++) {
      line(label + "_residual_" + i + "_m3", residuals[i]);
    }
    line(label + "_rms_residual_m3", result.getRmsResidual());
    line(label + "_condition_number", result.getConditionNumber());
  }

  /// The generalized material balance: the four fits, the scaling states and the refusal rows.
  private static void generalizedMb() {
    System.out.println("pvt_generalized_material_balance");

    materialBalanceFit("oil_and_gas", history(1.0e7, 2.0e9, false), "oil_and_gas", 0.0);
    materialBalanceFit("oil_and_gas_supported", history(1.0e7, 2.0e9, true), "oil_and_gas", 0.0);
    materialBalanceFit("initially_gas", history(0.0, 2.0e9, false), "initially_gas", 0.0);
    materialBalanceFit("initially_oil", history(1.0e7, 0.0, false), "initially_oil", 0.0);
    materialBalanceFit("known_gas_cap", history(1.0e7, 2.0e9, true), "known_gas_cap", 0.5);
    for (double scale : new double[] {1.0e-8, 1.0, 1.0e8}) {
      materialBalanceFit("scaled_" + scale, history(1.0e7 * scale, 2.0e9 * scale, true),
          "oil_and_gas", 0.0);
    }

    // The conventional black-oil limit at Rv = 0, computed from the same closure the test uses.
    Record initial = pvtRecord(300.0, 100.0, 1.5, 0.004, 0.0);
    GeneralizedMaterialBalance conventional = new GeneralizedMaterialBalance(initial);
    double expectedOil = 1.0e7;
    for (int i = 1; i <= 4; i++) {
      Record current = pvtRecord(300.0 - 40.0 * i, 100.0 - 15.0 * i, 1.5 - 0.04 * i,
          0.004 + 0.002 * i, 0.0);
      double expansion = OilMaterialBalance.eo(current.Bo, initial.Bo, initial.Rs, current.Rs,
          current.Bg);
      double np = 1.0e5 * i;
      double gp = (expectedOil * expansion - np * current.Bo) / current.Bg + np * current.Rs;
      conventional.addObservation(current, np, gp, 0.0, 0.0, 0.0);
    }
    materialBalanceFit("conventional_rv_zero", conventional, "initially_oil", 0.0);

    refusal("refused_collinear", () -> {
      GeneralizedMaterialBalance balance = new GeneralizedMaterialBalance(
          pvtRecord(300.0, 0.0, 1.0, 0.01, 0.0));
      balance.addObservation(pvtRecord(200.0, 0.0, 1.1, 0.02, 0.0), 1.0e5, 1.0e6, 0.0, 0.0, 0.0);
      balance.addObservation(pvtRecord(100.0, 0.0, 1.2, 0.03, 0.0), 2.0e5, 2.0e6, 0.0, 0.0, 0.0);
      balance.fitOilAndGas();
    });
    refusal("refused_negative_production", () -> {
      GeneralizedMaterialBalance balance = new GeneralizedMaterialBalance(
          pvtRecord(300.0, 100.0, 1.6, 0.004, 0.0002));
      Record current = pvtRecord(200.0, 60.0, 1.4, 0.008, 0.00012);
      balance.addObservation(current, -1.0e6, -1.0e8, 0.0, 0.0, 0.0);
      balance.addObservation(current, -2.0e6, -2.0e8, 0.0, 0.0, 0.0);
      balance.fitInitiallyOil();
    });
    refusal("refused_exhausted_inventory", () -> {
      GeneralizedMaterialBalance balance = new GeneralizedMaterialBalance(
          pvtRecord(300.0, 0.0, 1.0, 0.004, 0.0));
      balance.addObservation(pvtRecord(200.0, 0.0, 1.0, 0.08, 0.0), 0.0, 5.0e8, 0.0, 0.0, 0.0);
      balance.addObservation(pvtRecord(150.0, 0.0, 1.0, 0.008, 0.0), 0.0, 3.0e9, 0.0, 0.0, 0.0);
      balance.fitInitiallyGas();
    });
    refusal("refused_near_critical_split",
        () -> new GeneralizedMaterialBalance(pvtRecord(300.0, 100.0, 1.6, 0.004, 0.01)));
    refusal("refused_nan_pressure",
        () -> new GeneralizedMaterialBalance(pvtRecord(Double.NaN, 100.0, 1.6, 0.004, 0.0)));
    refusal("refused_empty_history", () -> new GeneralizedMaterialBalance(
        pvtRecord(300.0, 100.0, 1.6, 0.004, 0.0002)).fitInitiallyGas());
    refusal("refused_negative_gas_cap", () -> history(1.0e7, 2.0e9, true).fitKnownGasCap(-1.0));
    refusal("refused_zero_withdrawal", () -> {
      GeneralizedMaterialBalance balance = new GeneralizedMaterialBalance(
          pvtRecord(300.0, 100.0, 1.6, 0.004, 0.0002));
      Record same = pvtRecord(300.0, 100.0, 1.6, 0.004, 0.0002);
      balance.addObservation(same, 0.0, 0.0, 0.0, 0.0, 0.0);
      balance.addObservation(same, 0.0, 0.0, 0.0, 0.0, 0.0);
      balance.fitInitiallyGas();
    });
  }

  /// The Corey and LET curve primitives, which are package-private and read by reflection.
  private static double relPermCurve(String family, double saturation, double first, double second,
      double third) throws ReflectiveOperationException {
    if (family.equals("corey")) {
      Method corey = RelativePermeabilityGenerator.class.getDeclaredMethod("coreyCurve",
          double.class, double.class);
      corey.setAccessible(true);
      return (double) corey.invoke(null, saturation, first);
    }
    Method let = RelativePermeabilityGenerator.class.getDeclaredMethod("letCurve", double.class,
        double.class, double.class, double.class);
    let.setAccessible(true);
    return (double) let.invoke(null, saturation, first, second, third);
  }

  /// Prints a generated relative-permeability table row by row.
  private static void relPermTable(String label, RelativePermeabilityGenerator generator) {
    System.out.println(label);
    Map<String, double[]> table = generator.generate();
    String[] columns = table.keySet().toArray(new String[0]);
    System.out.println("columns=" + String.join(",", columns));
    int rows = table.get(columns[0]).length;
    line("rows", rows);
    for (int i = 0; i < rows; i++) {
      for (String column : columns) {
        line(column.toLowerCase(java.util.Locale.ROOT) + "_" + i, table.get(column)[i]);
      }
    }
    double[] values = table.get(columns[0]);
    for (int i = 0; i < values.length; i++) {
      for (String column : columns) {
        if (!Double.isFinite(table.get(column)[i])) {
          throw new IllegalStateException("a table value is not finite");
        }
      }
    }
  }

  /// The relative-permeability table generator: the primitives, every table type and the boundaries.
  private static void relperm() throws ReflectiveOperationException {
    System.out.println("pvt_relative_permeability");

    double[] samples = {0.0, 0.25, 0.5, 0.75, 1.0};
    for (int i = 0; i < samples.length; i++) {
      line("corey_n1_" + i, relPermCurve("corey", samples[i], 1.0, 0.0, 0.0));
      line("corey_n2_" + i, relPermCurve("corey", samples[i], 2.0, 0.0, 0.0));
      line("let_sym_" + i, relPermCurve("let", samples[i], 2.0, 1.0, 2.0));
    }

    RelativePermeabilityGenerator swofCorey = new RelativePermeabilityGenerator();
    swofCorey.setTableType(RelPermTableType.SWOF);
    swofCorey.setModelFamily(RelPermModelFamily.COREY);
    swofCorey.setSwc(0.15);
    swofCorey.setSorw(0.20);
    swofCorey.setKroMax(1.0);
    swofCorey.setKrwMax(0.25);
    swofCorey.setNo(2.5);
    swofCorey.setNw(1.5);
    swofCorey.setRows(25);
    relPermTable("swof_corey_25", swofCorey);

    RelativePermeabilityGenerator swofCorey50 = new RelativePermeabilityGenerator();
    swofCorey50.setTableType(RelPermTableType.SWOF);
    swofCorey50.setSwc(0.10);
    swofCorey50.setSorw(0.15);
    swofCorey50.setNo(3.0);
    swofCorey50.setNw(2.0);
    swofCorey50.setRows(50);
    relPermTable("swof_corey_50", swofCorey50);

    RelativePermeabilityGenerator swofLet = new RelativePermeabilityGenerator();
    swofLet.setTableType(RelPermTableType.SWOF);
    swofLet.setModelFamily(RelPermModelFamily.LET);
    swofLet.setSwc(0.15);
    swofLet.setSorw(0.20);
    swofLet.setKroMax(1.0);
    swofLet.setKrwMax(0.3);
    swofLet.setLo(2.5);
    swofLet.setEo(1.25);
    swofLet.setTo(1.75);
    swofLet.setLw(1.2);
    swofLet.setEw(1.5);
    swofLet.setTw(2.0);
    swofLet.setRows(30);
    relPermTable("swof_let_30", swofLet);

    RelativePermeabilityGenerator sgof = new RelativePermeabilityGenerator();
    sgof.setTableType(RelPermTableType.SGOF);
    sgof.setSwc(0.20);
    sgof.setSorg(0.15);
    sgof.setKroMax(1.0);
    sgof.setKrgMax(0.8);
    sgof.setNg(2.0);
    sgof.setNog(3.0);
    sgof.setRows(20);
    relPermTable("sgof_corey_20", sgof);

    RelativePermeabilityGenerator sof3 = new RelativePermeabilityGenerator();
    sof3.setTableType(RelPermTableType.SOF3);
    sof3.setSwc(0.20);
    sof3.setSorw(0.15);
    sof3.setSorg(0.10);
    sof3.setKroMax(1.0);
    sof3.setNo(2.0);
    sof3.setNog(2.0);
    sof3.setRows(15);
    relPermTable("sof3_corey_15", sof3);

    RelativePermeabilityGenerator slgofDefault = new RelativePermeabilityGenerator();
    slgofDefault.setTableType(RelPermTableType.SLGOF);
    slgofDefault.setRows(20);
    relPermTable("slgof_default", slgofDefault);

    RelativePermeabilityGenerator slgof = new RelativePermeabilityGenerator();
    slgof.setTableType(RelPermTableType.SLGOF);
    slgof.setSwc(0.20);
    slgof.setSorg(0.15);
    slgof.setKroMax(1.0);
    slgof.setKrgMax(0.8);
    slgof.setNg(2.0);
    slgof.setNog(3.0);
    slgof.setRows(20);
    relPermTable("slgof_corey_20", slgof);

    RelativePermeabilityGenerator critical = new RelativePermeabilityGenerator();
    critical.setTableType(RelPermTableType.SWOF);
    critical.setSwc(0.10);
    critical.setSwcr(0.20);
    critical.setSorw(0.20);
    critical.setKrwMax(0.5);
    critical.setNw(2.0);
    critical.setNo(2.0);
    critical.setRows(50);
    relPermTable("swof_critical_swcr", critical);

    RelativePermeabilityGenerator defaults = new RelativePermeabilityGenerator();
    relPermTable("swof_default", defaults);

    for (RelPermTableType type : new RelPermTableType[] {RelPermTableType.SWOF,
        RelPermTableType.SGOF, RelPermTableType.SOF3, RelPermTableType.SLGOF}) {
      RelativePermeabilityGenerator small = new RelativePermeabilityGenerator();
      small.setTableType(type);
      small.setRows(5);
      relPermTable("small_" + type.name().toLowerCase(java.util.Locale.ROOT), small);
    }

    RelativePermeabilityGenerator smallLet = new RelativePermeabilityGenerator();
    smallLet.setTableType(RelPermTableType.SWOF);
    smallLet.setModelFamily(RelPermModelFamily.LET);
    smallLet.setSwc(0.15);
    smallLet.setSorw(0.20);
    smallLet.setKroMax(1.0);
    smallLet.setKrwMax(0.3);
    smallLet.setLo(2.5);
    smallLet.setEo(1.25);
    smallLet.setTo(1.75);
    smallLet.setLw(1.2);
    smallLet.setEw(1.5);
    smallLet.setTw(2.0);
    smallLet.setRows(5);
    relPermTable("small_swof_let", smallLet);

    RelativePermeabilityGenerator descending = new RelativePermeabilityGenerator();
    descending.setTableType(RelPermTableType.SWOF);
    descending.setSwc(0.90);
    descending.setSorw(0.20);
    descending.setRows(5);
    relPermTable("swof_descending_grid", descending);

    refusal("refused_two_rows", () -> {
      RelativePermeabilityGenerator tooSmall = new RelativePermeabilityGenerator();
      tooSmall.setRows(2);
    });
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
      case "aquifer":
        aquifer();
        break;
      case "fetkovich":
        fetkovich();
        break;
      case "generalized_mb":
        generalizedMb();
        break;
      case "relperm":
        relperm();
        break;
      case "cvd_mb":
        cvdMaterialBalance();
        break;
      case "density_consistency":
        gasDensityConsistency();
        break;
      case "separator_path":
        separatorPath();
        break;
      default:
        System.err.println("unknown subcommand: " + which);
        System.exit(2);
    }
  }
}
