#![allow(non_snake_case)]

pub use config::{FactorizationScheme, FlavorScheme};
pub use error::HoppetError;
use std::{
    slice,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use config::{BaseConfig, PDFConfig, QCDConfig, QEDConfig};

mod config;
mod error;
mod ffi;
#[cfg(feature = "lhapdf")]
mod lhapdf;

static HOPPET_LOCK: AtomicBool = AtomicBool::new(false);
static HOPPET_COUNTER: AtomicUsize = AtomicUsize::new(0);
static mut XFX: Option<Box<Arc<dyn Fn(f64, f64, &mut [f64; 13]) + Send + Sync>>> = None;

extern "C" fn xfx_wrap(x: *const f64, Q: *const f64, res: *mut f64) {
    unsafe {
        match XFX {
            None => unreachable!(),
            Some(ref xfx) => xfx(
                *x,
                *Q,
                slice::from_raw_parts_mut(res, 13).try_into().unwrap(),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HoppetConfig {
    base: BaseConfig,
    pdf: Option<PDFConfig>,
    qcd: Option<QCDConfig>,
    qed: Option<QEDConfig>,
}

impl HoppetConfig {
    pub fn new() -> Self {
        return HoppetConfig {
            base: BaseConfig::default(),
            pdf: None,
            qcd: None,
            qed: None,
        };
    }

    pub fn start(&mut self, dy: f64, nloop: i32) -> &mut Self {
        self.base.dy = dy;
        self.base.dlnlnQ = dy / 4.;
        self.base.nloop = nloop;
        return self;
    }

    pub fn start_extended(
        &mut self,
        ymax: f64,
        dy: f64,
        Qmin: f64,
        Qmax: f64,
        dlnlnQ: f64,
        nloop: i32,
        order: i32,
        factscheme: FactorizationScheme,
    ) -> &mut Self {
        self.base.ymax = ymax;
        self.base.dy = dy;
        self.base.Qmin = Qmin;
        self.base.Qmax = Qmax;
        self.base.dlnlnQ = dlnlnQ;
        self.base.nloop = nloop;
        self.base.interpolation_order = order;
        self.base.factorization_scheme = factscheme;
        return self;
    }

    pub fn y_lnlnQ_orders(&mut self, yorder: i32, lnlnQorder: i32) -> &mut Self {
        self.base.yorder = yorder;
        self.base.lnlnQorder = lnlnQorder;
        return self;
    }

    pub fn flavor_scheme(&mut self, scheme: FlavorScheme) -> &mut Self {
        if let FlavorScheme::Fixed { nf } = scheme {
            self.base.split_nf = nf;
        }
        self.base.flavor_scheme = scheme;
        return self;
    }

    pub fn split_nf(&mut self, nf: i32) -> &mut Self {
        self.base.split_nf = nf;
        return self;
    }

    pub fn set_coupling(&mut self, asQ0: f64, Q0alphas: f64, nloop: i32) -> &mut Self {
        self.qcd = Some(QCDConfig {
            alphas_Q: asQ0,
            Q: Q0alphas,
            nloop,
        });
        return self;
    }

    pub fn assign(
        &mut self,
        xfx: impl Fn(f64, f64, &mut [f64; 13]) + Send + Sync + 'static,
    ) -> &mut Self {
        self.pdf = Some(PDFConfig::Assign {
            xfx: Box::new(Arc::new(xfx)),
        });
        return self;
    }

    pub fn evolve(
        &mut self,
        asQ0: f64,
        Q0alphas: f64,
        nloop: i32,
        muR_Q: f64,
        xfx: impl Fn(f64, f64, &mut [f64; 13]) + Send + Sync + 'static,
        Q0pdf: f64,
    ) -> &mut Self {
        self.qcd = Some(QCDConfig {
            alphas_Q: asQ0,
            Q: Q0alphas,
            nloop,
        });
        self.pdf = Some(PDFConfig::Evolve {
            xfx: Box::new(Arc::new(xfx)),
            muR: muR_Q,
            Q0: Q0pdf,
        });
        return self;
    }

    pub fn set_qed(&mut self, use_qed: bool, use_qcd_qed: bool, use_Plq_nnlo: bool) -> &mut Self {
        self.qed = Some(QEDConfig {
            with_qed: use_qed,
            qcd_qed: use_qcd_qed,
            plq: use_Plq_nnlo,
        });
        return self;
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all, level = tracing::Level::DEBUG))]
    pub fn init(&mut self) -> Result<Hoppet, HoppetError> {
        if HOPPET_LOCK.load(Ordering::Relaxed) {
            return Err(HoppetError::AlreadyInitialized);
        }
        HOPPET_LOCK.store(true, Ordering::Relaxed);
        unsafe {
            if let Some(ref qed) = self.qed {
                #[cfg(feature = "tracing")]
                tracing::debug!(
                    "Enabling QED {} mixed QCD-QED terms and {} Pˡ𐞥 splitting functions",
                    if qed.qcd_qed { "with" } else { "without" },
                    if qed.plq { "with" } else { "without" },
                );
                ffi::hoppetSetQED_c(&qed.with_qed, &qed.qcd_qed, &qed.plq);
            }
            #[cfg(feature = "tracing")]
            tracing::debug!("Using flavor scheme {:?}", self.base.flavor_scheme);
            match self.base.flavor_scheme {
                FlavorScheme::Fixed { nf } => ffi::hoppetsetffn_(&nf),
                FlavorScheme::PoleMassVFN { mc, mb, mt } => {
                    ffi::hoppetsetpolemassvfn_(&mc, &mb, &mt)
                }
                FlavorScheme::MSbarVFN { mc, mb, mt } => ffi::hoppetsetmsbarmassvfn_(&mc, &mb, &mt),
            }
            #[cfg(feature = "tracing")]
            tracing::debug!(
                "Using interpolation order {} for y and {} for lnlnQ",
                self.base.yorder,
                self.base.lnlnQorder
            );
            ffi::hoppetsetylnlnqinterporders_(&self.base.yorder, &self.base.lnlnQorder);
            #[cfg(feature = "tracing")]
            tracing::debug!(
                ymax = &self.base.ymax,
                dy = &self.base.dy,
                Qmin = &self.base.Qmin,
                Qmax = &self.base.Qmax,
                dlnlnQ = &self.base.dlnlnQ,
                nloop = &self.base.nloop,
                interpolation_order = &self.base.interpolation_order,
                factorization_scheme = ?&self.base.factorization_scheme,
                "Starting Hoppet"
            );
            ffi::hoppetstartextended_(
                &self.base.ymax,
                &self.base.dy,
                &self.base.Qmin,
                &self.base.Qmax,
                &self.base.dlnlnQ,
                &self.base.nloop,
                &self.base.interpolation_order,
                &self.base.factorization_scheme.clone().into(),
            );
            match (&self.qcd, &self.pdf) {
                (None, None) => (),
                (None, Some(PDFConfig::Evolve { .. })) => unreachable!(),
                (None, Some(PDFConfig::Assign { xfx })) => {
                    XFX = Some(xfx.clone());
                    ffi::hoppetassign_(xfx_wrap)
                }
                (
                    Some(QCDConfig { alphas_Q, Q, nloop }),
                    Some(PDFConfig::Evolve { xfx, muR, Q0 }),
                ) => {
                    XFX = Some(xfx.clone());
                    ffi::hoppetevolve_(alphas_Q, Q, nloop, muR, xfx_wrap, Q0);
                }
                (Some(QCDConfig { alphas_Q, Q, nloop }), Some(PDFConfig::Assign { xfx })) => {
                    XFX = Some(xfx.clone());
                    ffi::hoppetsetcoupling_(alphas_Q, Q, nloop);
                    ffi::hoppetassign_(xfx_wrap);
                }
                (Some(QCDConfig { alphas_Q, Q, nloop }), None) => {
                    ffi::hoppetsetcoupling_(alphas_Q, Q, nloop);
                }
            }
        }
        HOPPET_COUNTER.fetch_add(1, Ordering::Relaxed);
        return Ok(Hoppet { conf: self.clone() });
    }
}

#[derive(Debug)]
pub struct Hoppet {
    conf: HoppetConfig,
}

impl Hoppet {
    pub fn alphaS(&self, Q: f64) -> f64 {
        unsafe {
            return ffi::hoppetalphas_(&Q);
        }
    }

    pub fn alphaQED(&self, Q: f64) -> f64 {
        unsafe {
            return ffi::hoppetalphaqed_(&Q);
        }
    }

    pub fn eval(&self, x: f64, Q: f64, f: &mut [f64]) {
        debug_assert!(if let Some(QEDConfig { with_qed, .. }) = self.conf.qed
            && with_qed
        {
            f.len() >= 18
        } else {
            f.len() >= 13
        });
        unsafe {
            return ffi::hoppeteval_(&x, &Q, f.as_mut_ptr());
        }
    }

    pub fn eval_split(&self, x: f64, Q: f64, iloop: i32, f: &mut [f64]) {
        debug_assert!(if let Some(QEDConfig { with_qed, .. }) = self.conf.qed
            && with_qed
        {
            f.len() >= 18
        } else {
            f.len() >= 13
        });
        unsafe {
            ffi::hoppetevalsplit_(&x, &Q, &iloop, &self.conf.base.split_nf, f.as_mut_ptr());
        }
    }
}

impl Drop for Hoppet {
    fn drop(&mut self) {
        if HOPPET_COUNTER.load(Ordering::Relaxed) == 1 {
            unsafe {
                ffi::hoppetdeleteall_();
                XFX = None;
            }
            HOPPET_LOCK.store(false, Ordering::Relaxed);
            HOPPET_COUNTER.store(0, Ordering::Relaxed);
        } else {
            HOPPET_COUNTER.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

impl Clone for Hoppet {
    fn clone(&self) -> Self {
        HOPPET_COUNTER.fetch_add(1, Ordering::Relaxed);
        return Self {
            conf: self.conf.clone(),
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::{FactorizationScheme, FlavorScheme, HoppetConfig, HoppetError};
    use rayon::prelude::*;

    #[test]
    fn alphaS_test() {
        let hop = HoppetConfig::new()
            .start_extended(
                19.,
                0.02,
                1.3,
                100000.0,
                0.005,
                2,
                -6,
                FactorizationScheme::MSbar,
            )
            .flavor_scheme(FlavorScheme::PoleMassVFN {
                mc: 1.3,
                mb: 4.75,
                mt: 200000.,
            })
            .set_coupling(0.118001, 91.1876, 2)
            .init()
            .unwrap();
        assert_eq!(hop.alphaS(40.), 0.13480420980850222);
    }

    #[test]
    fn double_init_test() {
        let hop = HoppetConfig::new().init().unwrap();
        match HoppetConfig::new().init() {
            Ok(_) => panic!("Acquired second Hoppet handle"),
            Err(e) => match e {
                HoppetError::AlreadyInitialized => (),
                _ => unreachable!(),
            },
        }
        drop(hop);
        match HoppetConfig::new().init() {
            Ok(_) => (),
            Err(e) => match e {
                HoppetError::AlreadyInitialized => panic!("Hoppet lock not correctly dropped"),
                _ => unreachable!(),
            },
        }
    }

    #[test]
    fn parallel_test() {
        let Q = (0..100).map(|i| 1. + i as f64 * 100.).collect::<Vec<_>>();
        let hop = HoppetConfig::new()
            .set_coupling(0.118001, 91.1876, 2)
            .init()
            .unwrap();
        let res = Q.iter().map(|q| hop.alphaS(*q)).collect::<Vec<_>>();
        let res_par = Q.par_iter().map(|q| hop.alphaS(*q)).collect::<Vec<_>>();
        assert_eq!(res, res_par);
    }

    #[test]
    fn eval_test() {
        let hop = HoppetConfig::new()
            .set_coupling(0.118001, 91.1876, 2)
            .assign(|_x, _Q, res| res.fill(0.5))
            .init()
            .unwrap();
        let mut buf = [0.; 13];
        hop.eval(0.5, 40., &mut buf);
    }

    #[test]
    fn drop_test() {
        let hp = HoppetConfig::new()
            .set_coupling(0.118001, 91.1876, 2)
            .init()
            .unwrap();
        println!("{}", hp.alphaS(40.));
        let hp2 = hp.clone();
        drop(hp2);
        println!("{}", hp.alphaS(40.));
    }
}
