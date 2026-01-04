#![allow(non_snake_case)]

#[derive(Debug, Clone)]
pub(crate) struct BaseConfig {
    pub(crate) nloop: i32,
    pub(crate) dy: f64,
    pub(crate) ymax: f64,
    pub(crate) Qmin: f64,
    pub(crate) Qmax: f64,
    pub(crate) dlnlnQ: f64,
    pub(crate) interpolation_order: i32,
    pub(crate) factorization_scheme: FactorizationScheme,
    pub(crate) flavor_scheme: FlavorScheme,
    pub(crate) split_nf: i32,
}

impl Default for BaseConfig {
    fn default() -> Self {
        return BaseConfig {
            nloop: 2,
            dy: 0.05,
            ymax: 12.,
            Qmin: 1.,
            Qmax: 28000.,
            dlnlnQ: 0.0125,
            interpolation_order: -6,
            factorization_scheme: FactorizationScheme::MSbar,
            flavor_scheme: FlavorScheme::default(),
            split_nf: 5,
        };
    }
}

#[derive(Debug, Clone)]
pub(crate) enum PDFConfig {
    Assign {
        xfx: fn(f64, f64, &mut [f64; 13]),
    },
    Evolve {
        xfx: fn(f64, f64, &mut [f64; 13]),
        muR: f64,
        Q0: f64,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct QCDConfig {
    pub(crate) alphas_Q: f64,
    pub(crate) Q: f64,
    pub(crate) nloop: i32,
}

impl Default for QCDConfig {
    fn default() -> Self {
        return QCDConfig {
            alphas_Q: 0.118,
            Q: 91.2,
            nloop: 2,
        };
    }
}

#[derive(Debug, Clone)]
pub(crate) struct QEDConfig {
    pub(crate) with_qed: bool,
    pub(crate) qcd_qed: bool,
    pub(crate) plq: bool,
}

impl Default for QEDConfig {
    fn default() -> Self {
        return QEDConfig {
            with_qed: false,
            qcd_qed: false,
            plq: false,
        };
    }
}

#[derive(Default, Debug, Clone)]
pub enum FactorizationScheme {
    #[default]
    MSbar,
    DIS,
    PolMSbar,
    FragMSbar,
}

impl Into<i32> for FactorizationScheme {
    fn into(self) -> i32 {
        return match self {
            Self::MSbar => 1,
            Self::DIS => 2,
            Self::PolMSbar => 3,
            Self::FragMSbar => 4,
        };
    }
}

#[derive(Debug, Clone)]
pub enum FlavorScheme {
    Fixed { nf: i32 },
    PoleMassVFN { mc: f64, mb: f64, mt: f64 },
    MSbarVFN { mc: f64, mb: f64, mt: f64 },
}

impl Default for FlavorScheme {
    fn default() -> Self {
        return FlavorScheme::PoleMassVFN {
            mc: 1.414213563,
            mb: 4.5,
            mt: 175.0,
        };
    }
}
