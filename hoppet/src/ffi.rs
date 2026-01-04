#[allow(non_snake_case)]
#[allow(dead_code)]

unsafe extern "C" {

    pub(crate) fn hoppetSetQED_c(withqed: *const bool, qcdqed: *const bool, plq: *const bool);
    pub(crate) fn hoppetstart_(dy: *const f64, nloop: *const i32);
    pub(crate) fn hoppetstartextended_(
        ymax: *const f64,
        dy: *const f64,
        Qmin: *const f64,
        Qmax: *const f64,
        dlnlnQ: *const f64,
        nloop: *const i32,
        order: *const i32,
        factscheme: *const i32,
    );
    pub(crate) fn hoppetsetffn_(fixed_nf: *const i32);
    pub(crate) fn hoppetsetpolemassvfn_(mc: *const f64, mb: *const f64, mt: *const f64);
    pub(crate) fn hoppetsetmsbarmassvfn_(mc: *const f64, mb: *const f64, mt: *const f64);
    pub(crate) fn hoppetSetExactDGLAP_c(
        exact_nfthreshold: *const bool,
        exact_splitting: *const bool,
    );
    pub(crate) fn hoppetsetapproximatedglapn3lo_(splitting_variant: *const i32);
    pub(crate) fn hoppetsetsplittingnnlo_(splitting_variant: *const i32);
    pub(crate) fn hoppetsetsplittingn3lo_(splitting_variant: *const i32);
    pub(crate) fn hoppetsetn3lonfthresholds_(variant: *const i32);
    pub(crate) fn hoppetsetylnlnqinterporders_(yorder: *const i32, lnlnQorder: *const i32);
    pub(crate) fn hoppetassign_(xfx: extern "C" fn(x: *const f64, Q: *const f64, res: *mut f64));
    pub(crate) fn hoppetsetcoupling_(asQ0: *const f64, Q0alphas: *const f64, nloop: *const i32);
    pub(crate) fn hoppetevolve_(
        asQ0: *const f64,
        Q0alphas: *const f64,
        nloop: *const i32,
        muR_Q: *const f64,
        xfx: extern "C" fn(x: *const f64, Q: *const f64, res: *mut f64),
        Q0pdf: *const f64,
    );
    pub(crate) fn hoppetpreevolve_(
        asQ0: *const f64,
        Q0alphas: *const f64,
        nloop: *const i32,
        muR_Q: *const f64,
        Q0pdf: *const f64,
    );
    pub(crate) fn hoppetcachedevolve_(
        xfx: extern "C" fn(x: *const f64, Q: *const f64, res: *mut f64),
    );
    pub(crate) fn hoppetalphas_(Q: *const f64) -> f64;
    pub(crate) fn hoppetalphaqed_(Q: *const f64) -> f64;
    pub(crate) fn hoppeteval_(x: *const f64, Q: *const f64, f: *mut f64);
    pub(crate) fn hoppetevaliflv_(x: *const f64, Q: *const f64, iflv: *const i32) -> f64;
    pub(crate) fn hoppetevalsplit_(
        x: *const f64,
        Q: *const f64,
        iloop: *const i32,
        nf: *const i32,
        f: *mut f64,
    );
    pub(crate) fn hoppetdeleteall_();
}

#[cfg(test)]
mod tests {

    #[test]
    fn ffi_test() {
        unsafe {
            super::hoppetstartextended_(&19., &0.02, &1.3, &100000.0, &0.005, &2, &-6, &1);
            super::hoppetsetpolemassvfn_(&1.3, &4.75, &200000.);
            super::hoppetsetcoupling_(&0.118001, &91.1876, &2);
            assert_eq!(super::hoppetalphas_(&40.), 0.13480420980850222);
        }
    }
}
