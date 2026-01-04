#![allow(non_snake_case)]

pub use config::{FactorizationScheme, FlavorScheme};
pub use error::HoppetError;
use std::{
    slice,
    sync::atomic::{AtomicBool, Ordering},
};

use config::{BaseConfig, PDFConfig, QCDConfig, QEDConfig};

mod config;
mod error;
mod ffi;

static HOPPET_LOCK: AtomicBool = AtomicBool::new(false);
static mut XFX: Option<fn(f64, f64, &mut [f64; 13])> = None;

extern "C" fn xfx_wrap(x: *const f64, Q: *const f64, res: *mut f64) {
    unsafe {
        match XFX {
            None => unreachable!(),
            Some(xfx) => xfx(
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

    pub fn assign(&mut self, xfx: fn(f64, f64, &mut [f64; 13])) -> &mut Self {
        self.pdf = Some(PDFConfig::Assign { xfx });
        return self;
    }

    pub fn evolve(
        &mut self,
        asQ0: f64,
        Q0alphas: f64,
        nloop: i32,
        muR_Q: f64,
        xfx: fn(f64, f64, &mut [f64; 13]),
        Q0pdf: f64,
    ) -> &mut Self {
        self.qcd = Some(QCDConfig {
            alphas_Q: asQ0,
            Q: Q0alphas,
            nloop,
        });
        self.pdf = Some(PDFConfig::Evolve {
            xfx,
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

    pub fn init(&mut self) -> Result<Hoppet, HoppetError> {
        if HOPPET_LOCK.load(Ordering::Relaxed) {
            return Err(HoppetError::AlreadyInitialized);
        }
        HOPPET_LOCK.store(true, Ordering::Relaxed);
        unsafe {
            if let Some(ref qed) = self.qed {
                ffi::hoppetSetQED_c(&qed.with_qed, &qed.qcd_qed, &qed.plq);
            }
            match self.base.flavor_scheme {
                FlavorScheme::Fixed { nf } => ffi::hoppetsetffn_(&nf),
                FlavorScheme::PoleMassVFN { mc, mb, mt } => {
                    ffi::hoppetsetpolemassvfn_(&mc, &mb, &mt)
                }
                FlavorScheme::MSbarVFN { mc, mb, mt } => ffi::hoppetsetmsbarmassvfn_(&mc, &mb, &mt),
            }
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
                    XFX = Some(*xfx);
                    ffi::hoppetassign_(xfx_wrap)
                }
                (
                    Some(QCDConfig { alphas_Q, Q, nloop }),
                    Some(PDFConfig::Evolve { xfx, muR, Q0 }),
                ) => {
                    XFX = Some(*xfx);
                    ffi::hoppetevolve_(alphas_Q, Q, nloop, muR, xfx_wrap, Q0);
                }
                (Some(QCDConfig { alphas_Q, Q, nloop }), Some(PDFConfig::Assign { xfx })) => {
                    XFX = Some(*xfx);
                    ffi::hoppetsetcoupling_(alphas_Q, Q, nloop);
                    ffi::hoppetassign_(xfx_wrap);
                }
                (Some(QCDConfig { alphas_Q, Q, nloop }), None) => {
                    ffi::hoppetsetcoupling_(alphas_Q, Q, nloop);
                }
            }
        }
        return Ok(Hoppet {
            conf: self.clone(),
            //_phantom_unsync: std::marker::PhantomData,
            //_phantom_unsend: std::marker::PhantomData,
        });
    }
}

#[derive(Debug)]
pub struct Hoppet {
    conf: HoppetConfig,
    // The underlying Fortran library is not thread safe, therefore this type is not allowed to be Sync or Send. The
    // following phantom markers enforce this
    //_phantom_unsync: std::marker::PhantomData<std::cell::Cell<()>>,
    //_phantom_unsend: std::marker::PhantomData<std::sync::MutexGuard<'static, ()>>,
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
        unsafe {
            ffi::hoppetdeleteall_();
            XFX = None;
        }
        HOPPET_LOCK.store(false, Ordering::Relaxed);
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
            },
        }
        drop(hop);
        match HoppetConfig::new().init() {
            Ok(_) => (),
            Err(e) => match e {
                HoppetError::AlreadyInitialized => panic!("Hoppet lock not correctly dropped"),
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
}
