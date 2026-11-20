//! The estimate basis, the AACE classes and the material-take-off row.

/// An AACE estimate class, with the accuracy band and default method the class carries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EstimateClass {
    /// Final estimate, vendor quotes.
    Class1,
    /// Detailed estimate, firm quantities.
    Class2,
    /// Budget or FEED estimate.
    Class3,
    /// Study estimate, equipment-factored.
    #[default]
    Class4,
    /// Concept screening estimate.
    Class5,
}

impl EstimateClass {
    /// The class number, 1 to 5.
    #[must_use]
    pub fn number(self) -> u8 {
        match self {
            Self::Class1 => 1,
            Self::Class2 => 2,
            Self::Class3 => 3,
            Self::Class4 => 4,
            Self::Class5 => 5,
        }
    }

    /// The maturity label.
    #[must_use]
    pub fn maturity(self) -> &'static str {
        match self {
            Self::Class1 => "Final",
            Self::Class2 => "Detailed",
            Self::Class3 => "Budget/FEED",
            Self::Class4 => "Study",
            Self::Class5 => "Concept",
        }
    }

    /// The lower accuracy fraction.
    #[must_use]
    pub fn low_accuracy_fraction(self) -> f64 {
        match self {
            Self::Class1 => -0.10,
            Self::Class2 => -0.15,
            Self::Class3 => -0.20,
            Self::Class4 => -0.30,
            Self::Class5 => -0.50,
        }
    }

    /// The upper accuracy fraction.
    #[must_use]
    pub fn high_accuracy_fraction(self) -> f64 {
        match self {
            Self::Class1 => 0.15,
            Self::Class2 => 0.20,
            Self::Class3 => 0.30,
            Self::Class4 => 0.50,
            Self::Class5 => 1.00,
        }
    }

    /// The default estimating method.
    #[must_use]
    pub fn default_method(self) -> &'static str {
        match self {
            Self::Class1 => "vendor-quote",
            Self::Class2 => "detailed-quantity-takeoff",
            Self::Class3 => "sized-equipment-and-bulk-mto",
            Self::Class4 => "equipment-factored",
            Self::Class5 => "capacity-factored",
        }
    }

    /// The class for a numeric AACE class, or `None` outside 1 to 5.
    #[must_use]
    pub fn from_number(number: u8) -> Option<Self> {
        match number {
            1 => Some(Self::Class1),
            2 => Some(Self::Class2),
            3 => Some(Self::Class3),
            4 => Some(Self::Class4),
            5 => Some(Self::Class5),
            _ => None,
        }
    }
}

/// The source and quality basis a cost estimate travels with.
#[derive(Clone, Debug, PartialEq)]
pub struct CostBasis {
    /// The AACE estimate class.
    pub estimate_class: EstimateClass,
    /// The estimating method identifier.
    pub estimating_method: String,
    /// The data source identifier.
    pub data_source: String,
    /// The currency code; the values are USD magnitudes unless stated.
    pub currency_code: String,
    /// The cost year.
    pub cost_year: i32,
    /// The location factor applied to the estimate.
    pub location_factor: f64,
    /// The location the factor is for.
    pub location_basis: String,
    /// Free-text notes.
    pub notes: String,
}

impl Default for CostBasis {
    fn default() -> Self {
        Self {
            estimate_class: EstimateClass::Class4,
            estimating_method: EstimateClass::Class4.default_method().to_string(),
            data_source: "correlation".to_string(),
            currency_code: "USD".to_string(),
            cost_year: 2025,
            location_factor: 1.0,
            location_basis: "US Gulf Coast".to_string(),
            notes: "Equipment-factored NeqSim screening/study estimate.".to_string(),
        }
    }
}

impl CostBasis {
    /// A basis for a stated class, its method following the class.
    #[must_use]
    pub fn with_class(estimate_class: EstimateClass) -> Self {
        Self {
            estimate_class,
            estimating_method: estimate_class.default_method().to_string(),
            ..Self::default()
        }
    }
}

/// One material-take-off line.
#[derive(Clone, Debug, PartialEq)]
pub struct MtoItem {
    /// The line's item description.
    pub item: String,
    /// The material, where the class states one.
    pub material: Option<String>,
    /// The count, where the line is a count rather than a weight.
    pub quantity: Option<f64>,
    /// The quantity's unit.
    pub unit: Option<String>,
    /// The weight in kilograms, where the line has one.
    pub weight_kg: Option<f64>,
    /// The line cost in USD.
    pub cost_usd: f64,
    /// The source of the allowance.
    pub source: Option<String>,
}
