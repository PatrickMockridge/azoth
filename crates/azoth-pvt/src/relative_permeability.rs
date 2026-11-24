//! `pvt.relative_permeability` - the Corey and LET saturation tables.
//!
//! Spec: `specs/models/pvt/relative_permeability.toml`. The arithmetic is
//! `pvtsimulation/reservoirproperties/relpermeability/RelativePermeabilityGenerator.java` at
//! the pin: four Eclipse table types over the two curve families, with the class's endpoint
//! overwrites and its saturation normalisations.

use azoth_core::{AzothError, Result, Warning};

/// One row of a generated table; only the columns the table type answers are present.
#[derive(Debug, Clone, PartialEq)]
pub struct RelativePermeabilityRow {
    /// Water saturation, for SWOF.
    pub sw: Option<f64>,
    /// Gas saturation, for SGOF.
    pub sg: Option<f64>,
    /// Oil saturation, for SOF3.
    pub so: Option<f64>,
    /// Liquid saturation, for SLGOF.
    pub sl: Option<f64>,
    /// Water relative permeability.
    pub krw: Option<f64>,
    /// Gas relative permeability.
    pub krg: Option<f64>,
    /// Oil relative permeability, water-oil system.
    pub krow: Option<f64>,
    /// Oil relative permeability, gas-oil system.
    pub krog: Option<f64>,
    /// Water-oil capillary pressure; the class hardcodes zero.
    pub pcow: Option<f64>,
    /// Gas-oil capillary pressure; the class hardcodes zero.
    pub pcog: Option<f64>,
}

/// Result of `pvt.relative_permeability`.
#[derive(Debug, Clone, PartialEq)]
pub struct RelativePermeabilityResult {
    /// The Eclipse table type: `SWOF`, `SGOF`, `SOF3` or `SLGOF`.
    pub table_type: String,
    /// The saturation column's name.
    pub saturation_name: String,
    /// The table's rows.
    pub table: Vec<RelativePermeabilityRow>,
    /// Caveats.
    pub warnings: Vec<Warning>,
}

/// The model family.
#[derive(Clone, Copy, PartialEq)]
enum Family {
    Corey,
    Let,
}

/// The table type.
#[derive(Clone, Copy, PartialEq)]
enum Table {
    Swof,
    Sgof,
    Sof3,
    Slgof,
}

/// The model's parameters, with the class's defaults filled in.
struct Parameters {
    table: Table,
    family: Family,
    rows: usize,
    swc: f64,
    swcr: f64,
    sorw: f64,
    sorg: f64,
    sgcr: f64,
    kro_max: f64,
    krw_max: f64,
    krg_max: f64,
    no: f64,
    nw: f64,
    ng: f64,
    nog: f64,
    lo: f64,
    eo: f64,
    to: f64,
    lw: f64,
    ew: f64,
    tw: f64,
    lg: f64,
    eg: f64,
    tg: f64,
    log: f64,
    eog: f64,
    tog: f64,
}

/// The relative-permeability table over the stated parameters.
///
/// # Errors
/// * [`AzothError::InvalidInput`] for an unknown table type or model family, fewer than
///   three rows, a non-finite parameter, or a non-finite table value.
#[allow(clippy::too_many_arguments)]
pub fn relative_permeability(
    table_type: Option<&str>,
    model_family: Option<&str>,
    rows: Option<u32>,
    swc: Option<f64>,
    critical_water_saturation: Option<f64>,
    sorw: Option<f64>,
    sorg: Option<f64>,
    sgcr: Option<f64>,
    kro_max: Option<f64>,
    krw_max: Option<f64>,
    krg_max: Option<f64>,
    no: Option<f64>,
    nw: Option<f64>,
    ng: Option<f64>,
    nog: Option<f64>,
    lo: Option<f64>,
    eo: Option<f64>,
    to: Option<f64>,
    lw: Option<f64>,
    ew: Option<f64>,
    tw: Option<f64>,
    lg: Option<f64>,
    eg: Option<f64>,
    tg: Option<f64>,
    log: Option<f64>,
    eog: Option<f64>,
    tog: Option<f64>,
) -> Result<RelativePermeabilityResult> {
    let parameters = Parameters {
        table: parse_table(table_type.unwrap_or("swof"))?,
        family: parse_family(model_family.unwrap_or("corey"))?,
        rows: rows.unwrap_or(20) as usize,
        swc: swc.unwrap_or(0.0),
        swcr: critical_water_saturation.unwrap_or(-1.0),
        sorw: sorw.unwrap_or(0.0),
        sorg: sorg.unwrap_or(0.0),
        sgcr: sgcr.unwrap_or(0.0),
        kro_max: kro_max.unwrap_or(1.0),
        krw_max: krw_max.unwrap_or(1.0),
        krg_max: krg_max.unwrap_or(1.0),
        no: no.unwrap_or(2.0),
        nw: nw.unwrap_or(2.0),
        ng: ng.unwrap_or(2.0),
        nog: nog.unwrap_or(2.0),
        lo: lo.unwrap_or(2.0),
        eo: eo.unwrap_or(1.0),
        to: to.unwrap_or(2.0),
        lw: lw.unwrap_or(2.0),
        ew: ew.unwrap_or(1.0),
        tw: tw.unwrap_or(2.0),
        lg: lg.unwrap_or(2.0),
        eg: eg.unwrap_or(1.0),
        tg: tg.unwrap_or(2.0),
        log: log.unwrap_or(2.0),
        eog: eog.unwrap_or(1.0),
        tog: tog.unwrap_or(2.0),
    };
    if parameters.rows < 3 {
        return Err(AzothError::invalid_input(
            "rows",
            format!(
                "the table needs at least three rows, and it was given {}",
                parameters.rows
            ),
        ));
    }
    parameters.check_finite()?;
    let result = match parameters.table {
        Table::Swof => parameters.swof()?,
        Table::Sgof => parameters.sgof()?,
        Table::Sof3 => parameters.sof3()?,
        Table::Slgof => parameters.slgof()?,
    };
    Ok(result)
}

fn parse_table(name: &str) -> Result<Table> {
    match name {
        "swof" => Ok(Table::Swof),
        "sgof" => Ok(Table::Sgof),
        "sof3" => Ok(Table::Sof3),
        "slgof" => Ok(Table::Slgof),
        other => Err(AzothError::invalid_input(
            "table_type",
            format!("{other:?} is not one of the four Eclipse table types"),
        )),
    }
}

fn parse_family(name: &str) -> Result<Family> {
    match name {
        "corey" => Ok(Family::Corey),
        "let" => Ok(Family::Let),
        other => Err(AzothError::invalid_input(
            "model_family",
            format!("{other:?} is not one of the two relative-permeability families"),
        )),
    }
}

impl Parameters {
    fn check_finite(&self) -> Result<()> {
        for (name, value) in [
            ("swc", self.swc),
            ("critical_water_saturation", self.swcr),
            ("sorw", self.sorw),
            ("sorg", self.sorg),
            ("sgcr", self.sgcr),
            ("kro_max", self.kro_max),
            ("krw_max", self.krw_max),
            ("krg_max", self.krg_max),
            ("no", self.no),
            ("nw", self.nw),
            ("ng", self.ng),
            ("nog", self.nog),
            ("lo", self.lo),
            ("eo", self.eo),
            ("to", self.to),
            ("lw", self.lw),
            ("ew", self.ew),
            ("tw", self.tw),
            ("lg", self.lg),
            ("eg", self.eg),
            ("tg", self.tg),
            ("log", self.log),
            ("eog", self.eog),
            ("tog", self.tog),
        ] {
            if !value.is_finite() {
                return Err(AzothError::invalid_input(
                    name,
                    "a relative-permeability parameter must be finite",
                ));
            }
        }
        Ok(())
    }

    fn swof(&self) -> Result<RelativePermeabilityResult> {
        let effective_swcr = if self.swcr < 0.0 { self.swc } else { self.swcr };
        let max_sw = 1.0 - self.sorw;
        let sw = linspace(self.swc, max_sw, self.rows);
        let mut krw = Vec::with_capacity(self.rows);
        let mut krow = Vec::with_capacity(self.rows);
        let pcow = vec![0.0; self.rows];
        for value in &sw {
            let swn_krw = normalize_water_for_krw(*value, effective_swcr, self.sorw);
            let swn_kro = normalize_water_for_krow(*value, self.swc, self.sorw);
            if self.family == Family::Let {
                krw.push(self.krw_max * let_curve(swn_krw, self.lw, self.ew, self.tw));
                krow.push(self.kro_max * let_curve(1.0 - swn_kro, self.lo, self.eo, self.to));
            } else {
                krw.push(self.krw_max * corey_curve(swn_krw, self.nw));
                krow.push(self.kro_max * corey_curve(1.0 - swn_kro, self.no));
            }
        }
        krw[0] = 0.0;
        krow[0] = self.kro_max;
        krw[self.rows - 1] = self.krw_max;
        krow[self.rows - 1] = 0.0;
        let table = (0..self.rows)
            .map(|index| RelativePermeabilityRow {
                sw: Some(sw[index]),
                sg: None,
                so: None,
                sl: None,
                krw: Some(krw[index]),
                krg: None,
                krow: Some(krow[index]),
                krog: None,
                pcow: Some(pcow[index]),
                pcog: None,
            })
            .collect();
        check_rows(table, "SWOF", "Sw")
    }

    fn sgof(&self) -> Result<RelativePermeabilityResult> {
        let max_sg = 1.0 - self.swc - self.sorg;
        let sg = linspace(0.0, max_sg, self.rows);
        let (mut krg, mut krog, pcog) = self.gas_curves(&sg);
        krg[0] = 0.0;
        krog[0] = self.kro_max;
        krg[self.rows - 1] = self.krg_max;
        krog[self.rows - 1] = 0.0;
        let table = (0..self.rows)
            .map(|index| RelativePermeabilityRow {
                sw: None,
                sg: Some(sg[index]),
                so: None,
                sl: None,
                krw: None,
                krg: Some(krg[index]),
                krow: None,
                krog: Some(krog[index]),
                pcow: None,
                pcog: Some(pcog[index]),
            })
            .collect();
        check_rows(table, "SGOF", "Sg")
    }

    fn sof3(&self) -> Result<RelativePermeabilityResult> {
        let min_so = self.sorw.max(self.sorg);
        let max_so = 1.0 - self.swc;
        let so = linspace(min_so, max_so, self.rows);
        let mut krow = Vec::with_capacity(self.rows);
        let mut krog = Vec::with_capacity(self.rows);
        for value in &so {
            let son_w = clamp((value - self.sorw) / (1.0 - self.swc - self.sorw));
            let son_g = clamp((value - self.sorg) / (1.0 - self.swc - self.sorg));
            if self.family == Family::Let {
                krow.push(self.kro_max * let_curve(son_w, self.lo, self.eo, self.to));
                krog.push(self.kro_max * let_curve(son_g, self.log, self.eog, self.tog));
            } else {
                krow.push(self.kro_max * corey_curve(son_w, self.no));
                krog.push(self.kro_max * corey_curve(son_g, self.nog));
            }
        }
        krow[0] = 0.0;
        krog[0] = 0.0;
        krow[self.rows - 1] = self.kro_max;
        krog[self.rows - 1] = self.kro_max;
        let table = (0..self.rows)
            .map(|index| RelativePermeabilityRow {
                sw: None,
                sg: None,
                so: Some(so[index]),
                sl: None,
                krw: None,
                krg: None,
                krow: Some(krow[index]),
                krog: Some(krog[index]),
                pcow: None,
                pcog: None,
            })
            .collect();
        check_rows(table, "SOF3", "So")
    }

    fn slgof(&self) -> Result<RelativePermeabilityResult> {
        let min_sl = self.swc + self.sorg;
        let sl = linspace(min_sl, 1.0, self.rows);
        let sg: Vec<f64> = sl.iter().map(|value| 1.0 - value).collect();
        let (mut krg, mut krog, pcog) = self.gas_curves(&sg);
        krg[0] = self.krg_max;
        krog[0] = 0.0;
        krg[self.rows - 1] = 0.0;
        krog[self.rows - 1] = self.kro_max;
        let table = (0..self.rows)
            .map(|index| RelativePermeabilityRow {
                sw: None,
                sg: None,
                so: None,
                sl: Some(sl[index]),
                krw: None,
                krg: Some(krg[index]),
                krow: None,
                krog: Some(krog[index]),
                pcow: None,
                pcog: Some(pcog[index]),
            })
            .collect();
        check_rows(table, "SLGOF", "Sl")
    }

    fn gas_curves(&self, sg: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let mut krg = Vec::with_capacity(self.rows);
        let mut krog = Vec::with_capacity(self.rows);
        let pcog = vec![0.0; self.rows];
        for value in sg {
            let sgn_krg = normalize_gas_for_krg(*value, self.sgcr, self.swc, self.sorg);
            let sgn_kro = normalize_gas_for_krog(*value, self.sgcr, self.swc, self.sorg);
            if self.family == Family::Let {
                krg.push(self.krg_max * let_curve(sgn_krg, self.lg, self.eg, self.tg));
                krog.push(self.kro_max * let_curve(1.0 - sgn_kro, self.log, self.eog, self.tog));
            } else {
                krg.push(self.krg_max * corey_curve(sgn_krg, self.ng));
                krog.push(self.kro_max * corey_curve(1.0 - sgn_kro, self.nog));
            }
        }
        (krg, krog, pcog)
    }
}

/// Builds the SGOF/SLGOF endpoint overwrites and the result.
fn check_rows(
    table: Vec<RelativePermeabilityRow>,
    table_type: &str,
    saturation_name: &str,
) -> Result<RelativePermeabilityResult> {
    for (index, row) in table.iter().enumerate() {
        for value in [
            row.sw, row.sg, row.so, row.sl, row.krw, row.krg, row.krow, row.krog, row.pcow,
            row.pcog,
        ]
        .into_iter()
        .flatten()
        {
            if !value.is_finite() {
                return Err(AzothError::invalid_input(
                    format!("table[{index}]"),
                    "a generated relative-permeability value is not finite",
                ));
            }
        }
    }
    Ok(RelativePermeabilityResult {
        table_type: table_type.to_string(),
        saturation_name: saturation_name.to_string(),
        table,
        warnings: Vec::new(),
    })
}

fn normalize_water_for_krw(sw: f64, swcr: f64, sorw: f64) -> f64 {
    let denom = 1.0 - swcr - sorw;
    if denom <= 0.0 {
        return 0.0;
    }
    clamp((sw - swcr) / denom)
}

fn normalize_water_for_krow(sw: f64, swc: f64, sorw: f64) -> f64 {
    let denom = 1.0 - swc - sorw;
    if denom <= 0.0 {
        return 0.0;
    }
    clamp((sw - swc) / denom)
}

fn normalize_gas_for_krg(sg: f64, sgcr: f64, swc: f64, sorg: f64) -> f64 {
    let denom = 1.0 - swc - sgcr - sorg;
    if denom <= 0.0 {
        return 0.0;
    }
    clamp((sg - sgcr) / denom)
}

fn normalize_gas_for_krog(sg: f64, _sgcr: f64, swc: f64, sorg: f64) -> f64 {
    let denom = 1.0 - swc - sorg;
    if denom <= 0.0 {
        return 0.0;
    }
    clamp(sg / denom)
}

fn clamp(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn linspace(start: f64, end: f64, count: usize) -> Vec<f64> {
    if count < 2 {
        return vec![start];
    }
    let step = (end - start) / (count as f64 - 1.0);
    let mut values: Vec<f64> = (0..count)
        .map(|index| start + index as f64 * step)
        .collect();
    values[count - 1] = end;
    values
}

fn corey_curve(saturation: f64, exponent: f64) -> f64 {
    clamp(saturation).powf(exponent)
}

fn let_curve(saturation: f64, l: f64, e: f64, t: f64) -> f64 {
    let s = clamp(saturation);
    if s <= 0.0 {
        return 0.0;
    }
    if s >= 1.0 {
        return 1.0;
    }
    let numerator = s.powf(l);
    let denominator = numerator + e * (1.0 - s).powf(t);
    if denominator <= 0.0 {
        return 0.0;
    }
    numerator / denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_swof_table_is_twenty_rows_with_the_endpoint_overwrites() {
        let result = relative_permeability(
            None, None, None, None, None, None, None, None, None, None, None, None, None, None,
            None, None, None, None, None, None, None, None, None, None, None, None, None,
        )
        .unwrap();
        assert_eq!(result.table_type, "SWOF");
        assert_eq!(result.saturation_name, "Sw");
        assert_eq!(result.table.len(), 20);
        assert_eq!(result.table[0].sw, Some(0.0));
        assert_eq!(result.table[0].krw, Some(0.0));
        assert_eq!(result.table[0].krow, Some(1.0));
        assert_eq!(result.table[19].sw, Some(1.0));
        assert_eq!(result.table[19].krw, Some(1.0));
        assert_eq!(result.table[19].krow, Some(0.0));
    }

    #[test]
    fn fewer_than_three_rows_is_refused() {
        let error = relative_permeability(
            None,
            None,
            Some(2),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("three rows"));
    }

    #[test]
    fn a_descending_grid_is_published() {
        let result = relative_permeability(
            Some("swof"),
            None,
            Some(5),
            Some(0.9),
            None,
            Some(0.2),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert!(result.table[0].sw.unwrap() > result.table[4].sw.unwrap());
        assert_eq!(result.table[0].krw, Some(0.0));
        assert_eq!(result.table[4].krow, Some(0.0));
    }
}
