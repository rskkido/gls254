use super::{addcarry_u32, subborrow_u32, umull_add, umull_add2, sgnw};
use core::convert::TryFrom;

// Given integers k and n, with 0 <= k < n < Nmax (with n prime),
// return signed integers c0 and c1 such that k = c0/c1 mod n. Integers
// are provided as arrays of 32-bit limbs in little-endian convention
// (least significant limb comes first). This function is NOT
// constant-time and MUST NOT be used with secret inputs.
//
// Limit Nmax is such that the solution always exists; its value is:
//   Nmax = floor(2^254 / (2/sqrt(3)))
//        = 0x376CF5D0B09954E764AE85AE0F17077124BB06998A7B48F318E414C90DC8B4DC
//
// If a larger n is provided as parameter, then the algorithm still
// terminates, but the real (c0, c1) may be larger than 128 bits, and thus
// only truncated results are returned.
#[allow(dead_code)]
pub(crate) fn lagrange253_vartime(k: &[u32; 8], n: &[u32; 8]) -> (i128, i128) {
    let (v0, v1) = lagrange256_vartime(k, n, 254);
    let mut c0 = v0[3] as u128;
    let mut c1 = v1[3] as u128;
    for i in (0..3).rev() {
        c0 = (c0 << 32) | (v0[i] as u128);
        c1 = (c1 << 32) | (v1[i] as u128);
    }
    (c0 as i128, c1 as i128)
}

// ========================================================================

macro_rules! define_bigint { ($typename:ident, $bitlen:expr) => {

    #[derive(Clone, Copy, Debug)]
    struct $typename([u32; $typename::N]);

    #[allow(dead_code)]
    impl $typename {
        const BITLEN: usize = $bitlen;
        const N: usize = (Self::BITLEN + 31) >> 5;
        const ZERO: Self = Self([0u32; Self::N]);

        // Return true iff self < rhs (inputs must be nonnegative).
        fn lt(self, rhs: &Self) -> bool {
            let (_, mut cc) = subborrow_u32(self.0[0], rhs.0[0], 0);
            for i in 1..Self::N {
                (_, cc) = subborrow_u32(self.0[i], rhs.0[i], cc);
            }
            cc != 0
        }

        // Swap the contents of self with rhs.
        fn swap(&mut self, rhs: &mut Self) {
            for i in 0..Self::N {
                let t = self.0[i];
                self.0[i] = rhs.0[i];
                rhs.0[i] = t;
            }
        }

        // Get the length (in bits) of this value.
        fn bitlength(self) -> u32 {
            let m = sgnw(self.0[Self::N - 1]);
            for i in (0..Self::N).rev() {
                let aw = self.0[i] ^ m;
                if aw != 0 {
                    return 32 * (i as u32) + 32 - aw.leading_zeros();
                }
            }
            0
        }

        // Return true if self is lower than 2^(32*s - 1). The value self
        // MUST be non-negative. The value s MUST be greater than 0, and
        // not greater than Self::N.
        fn ltnw(self, s: usize) -> bool {
            for i in s..Self::N {
                if self.0[i] != 0 {
                    return false;
                }
            }
            self.0[s - 1] < 0x80000000
        }

        // Return true for negative values.
        fn is_negative(self) -> bool {
            self.0[Self::N - 1] >= 0x80000000
        }

        // Add (2^s)*rhs to self.
        fn set_add_shifted(&mut self, rhs: &Self, s: u32) {
            if s < 32 {
                if s == 0 {
                    let (d0, mut cc) = addcarry_u32(self.0[0], rhs.0[0], 0);
                    self.0[0] = d0;
                    for i in 1..Self::N {
                        let (dx, ee) = addcarry_u32(self.0[i], rhs.0[i], cc);
                        self.0[i] = dx;
                        cc = ee;
                    }
                } else {
                    let (d0, mut cc) = addcarry_u32(
                        self.0[0], rhs.0[0] << s, 0);
                    self.0[0] = d0;
                    for i in 1..Self::N {
                        let bw = (rhs.0[i - 1] >> (32 - s)) | (rhs.0[i] << s);
                        let (dx, ee) = addcarry_u32(self.0[i], bw, cc);
                        self.0[i] = dx;
                        cc = ee;
                    }
                }
            } else {
                let j = (s >> 5) as usize;
                if j >= Self::N {
                    return;
                }
                let s = s & 31;
                if s == 0 {
                    let (dj, mut cc) = addcarry_u32(self.0[j], rhs.0[0], 0);
                    self.0[j] = dj;
                    for i in (j + 1)..Self::N {
                        let (dx, ee) = addcarry_u32(
                            self.0[i], rhs.0[i - j], cc);
                        self.0[i] = dx;
                        cc = ee;
                    }
                } else {
                    let (dj, mut cc) = addcarry_u32(
                        self.0[j], rhs.0[0] << s, 0);
                    self.0[j] = dj;
                    for i in (j + 1)..Self::N {
                        let bw = (rhs.0[i - j - 1] >> (32 - s))
                            | (rhs.0[i - j] << s);
                        let (dx, ee) = addcarry_u32(self.0[i], bw, cc);
                        self.0[i] = dx;
                        cc = ee;
                    }
                }
            }
        }

        // Subtract (2^s)*rhs from self.
        fn set_sub_shifted(&mut self, rhs: &Self, s: u32) {
            if s < 32 {
                if s == 0 {
                    let (d0, mut cc) = subborrow_u32(self.0[0], rhs.0[0], 0);
                    self.0[0] = d0;
                    for i in 1..Self::N {
                        let (dx, ee) = subborrow_u32(self.0[i], rhs.0[i], cc);
                        self.0[i] = dx;
                        cc = ee;
                    }
                } else {
                    let (d0, mut cc) = subborrow_u32(
                        self.0[0], rhs.0[0] << s, 0);
                    self.0[0] = d0;
                    for i in 1..Self::N {
                        let bw = (rhs.0[i - 1] >> (32 - s)) | (rhs.0[i] << s);
                        let (dx, ee) = subborrow_u32(self.0[i], bw, cc);
                        self.0[i] = dx;
                        cc = ee;
                    }
                }
            } else {
                let j = (s >> 5) as usize;
                if j >= Self::N {
                    return;
                }
                let s = s & 31;
                if s == 0 {
                    let (dj, mut cc) = subborrow_u32(self.0[j], rhs.0[0], 0);
                    self.0[j] = dj;
                    for i in (j + 1)..Self::N {
                        let (dx, ee) = subborrow_u32(
                            self.0[i], rhs.0[i - j], cc);
                        self.0[i] = dx;
                        cc = ee;
                    }
                } else {
                    let (dj, mut cc) = subborrow_u32(
                        self.0[j], rhs.0[0] << s, 0);
                    self.0[j] = dj;
                    for i in (j + 1)..Self::N {
                        let bw = (rhs.0[i - j - 1] >> (32 - s))
                            | (rhs.0[i - j] << s);
                        let (dx, ee) = subborrow_u32(self.0[i],bw, cc);
                        self.0[i] = dx;
                        cc = ee;
                    }
                }
            }
        }
    }

} } // End of macro: define_bigint

macro_rules! define_lagrange { ($name:ident, $n0:ident, $n1:ident, $n2:ident, $n3:ident) => {

    #[allow(dead_code)]
    pub(crate) fn $name(k: &[u32; $n1::N], n: &[u32; $n1::N], max_bitlen: u32)
        -> ([u32; $n0::N], [u32; $n0::N])
    {
        // Product of integers. Operands must be non-negative.
        fn umul(a: &[u32; $n1::N], b: &[u32; $n1::N]) -> $n3 {
            let mut d = $n3::ZERO;
            for i in 0..$n1::N {
                let (lo, mut cc) = umull_add(a[i], b[0], d.0[i]);
                d.0[i] = lo;
                for j in 1..$n1::N {
                    if (i + j) >= $n3::N {
                        break;
                    }
                    let (lo, hi) = umull_add2(a[i], b[j], d.0[i + j], cc);
                    d.0[i + j] = lo;
                    cc = hi;
                }
                if (i + $n1::N) < $n3::N {
                    d.0[i + $n1::N] = cc;
                }
            }
            d
        }

        // Initialization.
        // Coordinates of u and v are truncated (type $n0) since after
        // reduction, they should fit. Values nu (norm of u), nv (norm of v)
        // and sp (scalar product of u and v) are full-size.

        // u <- [n, 0]
        let mut u0 = $n0::ZERO;
        u0.0[..].copy_from_slice(&n[..$n0::N]);
        let mut u1 = $n0::ZERO;

        // v <- [k, 1]
        let mut v0 = $n0::ZERO;
        v0.0[..].copy_from_slice(&k[..$n0::N]);
        let mut v1 = $n0::ZERO;
        v1.0[0] = 1;

        // nu = u0^2 + u1^2 = n^2
        let mut nu = umul(n, n);

        // nv = v0^2 + v1^2 = k^2 + 1
        let mut nv = umul(k, k);
        let (dx, mut cc) = addcarry_u32(nv.0[0], 1, 0);
        nv.0[0] = dx;
        for i in 1..$n3::N {
            let (dx, ee) = addcarry_u32(nv.0[i], 0, cc);
            nv.0[i] = dx;
            cc = ee;
        }

        // sp = u0*v0 + u1*v1 = n*k
        let mut sp = umul(n, k);

        // We use a flag to indicate the first iteration, because at that
        // iteration, sp might lack a sign bit (it's 0, due to initial
        // conditions, but the unsigned value might fill the complete type).
        // After the first iteration, sp is necessarily lower than n/2 and
        // there is room for the sign bit.
        let mut first = true;

        // First algorithm loop, to shrink values enough to fit in type $n2.
        loop {
            // If u is smaller than v, then swap u and v.
            if nu.lt(&nv) {
                u0.swap(&mut v0);
                u1.swap(&mut v1);
                nu.swap(&mut nv);
            }

            // If nu has shrunk enough, then we can switch to the
            // second loop (since v is smaller than u at this point).
            if nu.ltnw($n2::N) {
                break;
            }

            // If v is small enough, return it.
            let bl_nv = nv.bitlength();
            if bl_nv <= max_bitlen {
                return (v0.0, v1.0);
            }

            // Compute this amount s = len(sp) - len(nv)
            // (if s < 0, it is replaced with 0).
            let bl_sp = sp.bitlength();
            let mut s = bl_sp.wrapping_sub(bl_nv);
            s &= !(((s as i32) >> 31) as u32);

            // Subtract or add v*2^s from/to u, depending on the sign of sp.
            if first || !sp.is_negative() {
                first = false;
                u0.set_sub_shifted(&v0, s);
                u1.set_sub_shifted(&v1, s);
                nu.set_add_shifted(&nv, 2 * s);
                nu.set_sub_shifted(&sp, s + 1);
                sp.set_sub_shifted(&nv, s);
            } else {
                u0.set_add_shifted(&v0, s);
                u1.set_add_shifted(&v1, s);
                nu.set_add_shifted(&nv, 2 * s);
                nu.set_add_shifted(&sp, s + 1);
                sp.set_add_shifted(&nv, s);
            }
        }

        // Shrink nu, nv and sp to the shorter size of $n2
        let mut new_nu = $n2::ZERO;
        let mut new_nv = $n2::ZERO;
        let mut new_sp = $n2::ZERO;
        new_nu.0[..].copy_from_slice(&nu.0[..$n2::N]);
        new_nv.0[..].copy_from_slice(&nv.0[..$n2::N]);
        new_sp.0[..].copy_from_slice(&sp.0[..$n2::N]);
        let mut nu = new_nu;
        let mut nv = new_nv;
        let mut sp = new_sp;

        // In the secondary loop, we need to check for the end condition,
        // which can be a "stuck" value of sp.
        let mut last_bl_sp = sp.bitlength();
        let mut stuck = 0u32;

        // Second algorithm loop, once values have shrunk enough to fit in $n2.
        loop {
            // If u is smaller than v, then swap u and v.
            if nu.lt(&nv) {
                u0.swap(&mut v0);
                u1.swap(&mut v1);
                nu.swap(&mut nv);
            }

            // If v is small enough, return it.
            let bl_nv = nv.bitlength();
            if bl_nv <= max_bitlen {
                return (v0.0, v1.0);
            }

            // sp normally decreases by 1 bit every two iterations. If it
            // appears to be "stuck" for too long, then this means that
            // we have reached the end of the algorithm, which implies that
            // the target bit length for nv, tested above, was not reached;
            // this means that the function was parameterized too eagerly.
            // It is up to the caller to handle all possible cases (some
            // callers can be made to tolerate truncated (v0,v1)).
            let bl_sp = sp.bitlength();
            if bl_sp >= last_bl_sp {
                stuck += 1;
                if bl_sp > last_bl_sp || stuck > 3 {
                    return (v0.0, v1.0);
                }
            } else {
                last_bl_sp = bl_sp;
                stuck = 0;
            }

            // s = len(sp) - len(nv)
            // (if s < 0, it is replaced with 0).
            let mut s = bl_sp.wrapping_sub(bl_nv);
            s &= !(((s as i32) >> 31) as u32);

            // Subtract or add v*2^s from/to u, depending on the sign of sp.
            if first || !sp.is_negative() {
                first = false;
                u0.set_sub_shifted(&v0, s);
                u1.set_sub_shifted(&v1, s);
                nu.set_add_shifted(&nv, 2 * s);
                nu.set_sub_shifted(&sp, s + 1);
                sp.set_sub_shifted(&nv, s);
            } else {
                u0.set_add_shifted(&v0, s);
                u1.set_add_shifted(&v1, s);
                nu.set_add_shifted(&nv, 2 * s);
                nu.set_add_shifted(&sp, s + 1);
                sp.set_add_shifted(&nv, s);
            }
        }
    }

} } // End of macro: define_lagrange

define_bigint!(ZInt128, 128);
define_bigint!(ZInt192, 192);
define_bigint!(ZInt256, 256);
define_bigint!(ZInt320, 320);
define_bigint!(ZInt384, 384);
define_bigint!(ZInt448, 448);
define_bigint!(ZInt512, 512);
define_bigint!(ZInt640, 640);
define_bigint!(ZInt768, 768);
define_bigint!(ZInt896, 896);
define_bigint!(ZInt1024, 1024);

define_lagrange!(lagrange256_vartime, ZInt128, ZInt256, ZInt384, ZInt512);
define_lagrange!(lagrange320_vartime, ZInt192, ZInt320, ZInt448, ZInt640);
define_lagrange!(lagrange384_vartime, ZInt192, ZInt384, ZInt512, ZInt768);
define_lagrange!(lagrange448_vartime, ZInt256, ZInt448, ZInt640, ZInt896);
define_lagrange!(lagrange512_vartime, ZInt256, ZInt512, ZInt768, ZInt1024);

//
// Rules:
//   k and n must have the same length, which is between 8 and 16 (inclusive)
//   k and n use unsigned little-endian notation
//   k < n (numerically)
//   c0 and c1 must have length at most ceil(n.len()/2)
// Processing ends when the minimal-size vector has been found, or when
// a vector v such that ||v||^2 < 2^max_bitlen has been found, whichever
// comes first.
// If the minimal-size vector does not fit in (c0,c1) then it is truncated.
// c0 and c1 use _signed_ little-endian notation.
#[allow(dead_code)]
pub(crate) fn lagrange_vartime(k: &[u32], n: &[u32], max_bitlen: u32,
    c0: &mut [u32], c1: &mut [u32])
{
    if n.len() < 8 || n.len() > 16 {
        unimplemented!();
    }
    // Expand k and n into larger arrays so that we may have an even number
    // of limbs.
    let mut nk = [0u32; 16];
    let mut nn = [0u32; 16];
    nk[..k.len()].copy_from_slice(k);
    nn[..n.len()].copy_from_slice(n);
    let nlen = (n.len() + 1) & !1usize;
    let k = &nk[..nlen];
    let n = &nn[..nlen];
    match nlen {
        8 => {
            let (v0, v1) = lagrange256_vartime(
                <&[u32; 8]>::try_from(k).unwrap(),
                <&[u32; 8]>::try_from(n).unwrap(),
                max_bitlen);
            c0.copy_from_slice(&v0[..c0.len()]);
            c1.copy_from_slice(&v1[..c1.len()]);
        }
        10 => {
            let (v0, v1) = lagrange320_vartime(
                <&[u32; 10]>::try_from(k).unwrap(),
                <&[u32; 10]>::try_from(n).unwrap(),
                max_bitlen);
            c0.copy_from_slice(&v0[..c0.len()]);
            c1.copy_from_slice(&v1[..c1.len()]);
        }
        12 => {
            let (v0, v1) = lagrange384_vartime(
                <&[u32; 12]>::try_from(k).unwrap(),
                <&[u32; 12]>::try_from(n).unwrap(),
                max_bitlen);
            c0.copy_from_slice(&v0[..c0.len()]);
            c1.copy_from_slice(&v1[..c1.len()]);
        }
        14 => {
            let (v0, v1) = lagrange448_vartime(
                <&[u32; 14]>::try_from(k).unwrap(),
                <&[u32; 14]>::try_from(n).unwrap(),
                max_bitlen);
            c0.copy_from_slice(&v0[..c0.len()]);
            c1.copy_from_slice(&v1[..c1.len()]);
        }
        16 => {
            let (v0, v1) = lagrange512_vartime(
                <&[u32; 16]>::try_from(k).unwrap(),
                <&[u32; 16]>::try_from(n).unwrap(),
                max_bitlen);
            c0.copy_from_slice(&v0[..c0.len()]);
            c1.copy_from_slice(&v1[..c1.len()]);
        }
        _ => {
            unimplemented!();
        }
    }
}
