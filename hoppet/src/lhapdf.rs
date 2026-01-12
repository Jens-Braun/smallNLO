use crate::{FlavorScheme, Hoppet, HoppetConfig, HoppetError};

impl HoppetConfig {
    pub fn init_from_lhaid(id: i32) -> Result<Hoppet, HoppetError> {
        return Self::init_from_pdf(lhapdf::Pdf::with_lhaid(id)?);
    }

    pub fn init_from_lhapdf(set: &str, member: i32) -> Result<Hoppet, HoppetError> {
        return Self::init_from_pdf(lhapdf::Pdf::with_setname_and_member(set, member)?);
    }

    pub fn init_from_pdf(mut pdf: lhapdf::Pdf) -> Result<Hoppet, HoppetError> {
        let ymax = pdf.x_min().recip().ln().ceil();
        let dy = 0.05;
        let dlnlnQ = if ymax > 15. { dy / 8. } else { dy / 4. };
        let qmin = pdf.q_min();
        let qmax = pdf.q_max();
        let nloop = pdf.order_qcd() + 1;
        return Self::new()
            .start_extended(
                ymax,
                dy,
                qmin,
                qmax,
                dlnlnQ,
                nloop,
                -6,
                crate::FactorizationScheme::MSbar,
            )
            .y_lnlnQ_orders(2, 2)
            .flavor_scheme(FlavorScheme::PoleMassVFN {
                mc: pdf.quark_mass(4),
                mb: pdf.quark_mass(5),
                mt: if pdf.has_flavor(6) {
                    pdf.quark_mass(6)
                } else {
                    2. * qmax
                },
            })
            .set_coupling(pdf.alphas_q2(qmin * qmin), qmin, nloop)
            .assign(move |x, Q, res| {
                res[0] = pdf.xfx_q2(-6, x, Q * Q);
                res[1] = pdf.xfx_q2(-5, x, Q * Q);
                res[2] = pdf.xfx_q2(-4, x, Q * Q);
                res[3] = pdf.xfx_q2(-3, x, Q * Q);
                res[4] = pdf.xfx_q2(-2, x, Q * Q);
                res[5] = pdf.xfx_q2(-1, x, Q * Q);
                res[6] = pdf.xfx_q2(21, x, Q * Q);
                res[7] = pdf.xfx_q2(1, x, Q * Q);
                res[8] = pdf.xfx_q2(2, x, Q * Q);
                res[9] = pdf.xfx_q2(3, x, Q * Q);
                res[10] = pdf.xfx_q2(4, x, Q * Q);
                res[11] = pdf.xfx_q2(5, x, Q * Q);
                res[12] = pdf.xfx_q2(6, x, Q * Q);
            })
            .init();
    }
}

#[cfg(test)]
mod tests {
    use crate::HoppetConfig;

    #[test]
    fn lhapdf_test() {
        let hop = HoppetConfig::init_from_lhapdf("CT10nlo", 0).unwrap();
        let mut buf = [0.; 13];
        hop.eval(0.1, 20., &mut buf);
        assert_eq!(buf[6], 1.0293408391162413);
    }
}
