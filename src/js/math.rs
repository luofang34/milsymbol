//! `Math.sin` / `Math.cos` exactly as V8 computes them.
//!
//! V8 (Node 26, V8 14.6, built without `V8_USE_LIBM_TRIG_FUNCTIONS`) uses its
//! own port of fdlibm (`src/base/ieee754.cc`). Other libms, including the
//! `libm` crate, differ from it in the last bit for some arguments, which
//! changes speed-leader and direction-arrow coordinates in the SVG output.
//! This is a line-for-line port of V8's `sin`, `cos`, `__kernel_sin`,
//! `__kernel_cos`, `__ieee754_rem_pio2` and `__kernel_rem_pio2`.
//!
//! V8's arm64 builds are compiled with floating-point contraction, so their
//! results differ in the last bit from x64 builds. With `FUSED` every
//! expression clang contracts is evaluated as a fused multiply-add
//! ([`mad`]); without it, evaluation is plain IEEE (x64 builds and WebAssembly).

/// `a * b + c`, fused (single rounding) when `FUSED`.
fn mad<const FUSED: bool>(a: f64, b: f64, c: f64) -> f64 {
    if FUSED { libm::fma(a, b, c) } else { a * b + c }
}

fn high_word(x: f64) -> i32 {
    (x.to_bits() >> 32) as u32 as i32
}

fn low_word(x: f64) -> u32 {
    (x.to_bits() & 0xFFFF_FFFF) as u32
}

fn from_words(hi: i32, lo: u32) -> f64 {
    f64::from_bits((u64::from(hi as u32) << 32) | u64::from(lo))
}

fn with_high_word(x: f64, hi: i32) -> f64 {
    from_words(hi, low_word(x))
}

/// `sin(x)` as V8's `Math.sin`.
pub(crate) fn sin<const F: bool>(x: f64) -> f64 {
    let ix = high_word(x) & 0x7FFF_FFFF;
    if ix <= 0x3FE9_21FB {
        return kernel_sin::<F>(x, 0.0, false);
    }
    if ix >= 0x7FF0_0000 {
        return f64::NAN;
    }
    let (n, y0, y1) = rem_pio2::<F>(x);
    match n & 3 {
        0 => kernel_sin::<F>(y0, y1, true),
        1 => kernel_cos::<F>(y0, y1),
        2 => -kernel_sin::<F>(y0, y1, true),
        _ => -kernel_cos::<F>(y0, y1),
    }
}

/// `cos(x)` as V8's `Math.cos`.
pub(crate) fn cos<const F: bool>(x: f64) -> f64 {
    let ix = high_word(x) & 0x7FFF_FFFF;
    if ix <= 0x3FE9_21FB {
        return kernel_cos::<F>(x, 0.0);
    }
    if ix >= 0x7FF0_0000 {
        return f64::NAN;
    }
    let (n, y0, y1) = rem_pio2::<F>(x);
    match n & 3 {
        0 => kernel_cos::<F>(y0, y1),
        1 => -kernel_sin::<F>(y0, y1, true),
        2 => -kernel_cos::<F>(y0, y1),
        _ => kernel_sin::<F>(y0, y1, true),
    }
}

fn kernel_cos<const F: bool>(x: f64, y: f64) -> f64 {
    const C1: f64 = f64::from_bits(0x3FA555555555554C); // 4.16666666666666019037e-02
    const C2: f64 = f64::from_bits(0xBF56C16C16C15177); // -1.38888888888741095749e-03
    const C3: f64 = f64::from_bits(0x3EFA01A019CB1590); // 2.48015872894767294178e-05
    const C4: f64 = f64::from_bits(0xBE927E4F809C52AD); // -2.75573143513906633035e-07
    const C5: f64 = f64::from_bits(0x3E21EE9EBDB4B1C4); // 2.08757232129817482790e-09
    const C6: f64 = f64::from_bits(0xBDA8FAE9BE8838D4); // -1.13596475577881948265e-11
    let ix = high_word(x) & 0x7FFF_FFFF;
    if ix < 0x3E40_0000 && (x as i32) == 0 {
        return 1.0;
    }
    let z = x * x;
    let m = mad::<F>;
    let r = z * m(z, m(z, m(z, m(z, m(z, C6, C5), C4), C3), C2), C1);
    let zr_xy = m(z, r, -(x * y));
    if ix < 0x3FD3_3333 {
        return 1.0 - m(0.5, z, -zr_xy);
    }
    let qx = if ix > 0x3FE9_0000 {
        0.28125
    } else {
        from_words(ix - 0x0020_0000, 0)
    };
    let iz = m(0.5, z, -qx);
    let a = 1.0 - qx;
    a - (iz - zr_xy)
}

fn kernel_sin<const F: bool>(x: f64, y: f64, iy: bool) -> f64 {
    const S1: f64 = f64::from_bits(0xBFC5555555555549); // -1.66666666666666324348e-01
    const S2: f64 = f64::from_bits(0x3F8111111110F8A6); // 8.33333333332248946124e-03
    const S3: f64 = f64::from_bits(0xBF2A01A019C161D5); // -1.98412698298579493134e-04
    const S4: f64 = f64::from_bits(0x3EC71DE357B1FE7D); // 2.75573137070700676789e-06
    const S5: f64 = f64::from_bits(0xBE5AE5E68A2B9CEB); // -2.50507602534068634195e-08
    const S6: f64 = f64::from_bits(0x3DE5D93A5ACFD57C); // 1.58969099521155010221e-10
    let ix = high_word(x) & 0x7FFF_FFFF;
    if ix < 0x3E40_0000 && (x as i32) == 0 {
        return x;
    }
    let z = x * x;
    let v = z * x;
    let m = mad::<F>;
    let r = m(z, m(z, m(z, m(z, S6, S5), S4), S3), S2);
    if !iy {
        m(v, m(z, r, S1), x)
    } else {
        let inner = m(z, m(0.5, y, -(v * r)), -y);
        x - m(-v, S1, inner)
    }
}

/// 396 hex digits of 2/π in 24-bit chunks.
const TWO_OVER_PI: [i32; 66] = [
    0xA2F983, 0x6E4E44, 0x1529FC, 0x2757D1, 0xF534DD, 0xC0DB62, 0x95993C, 0x439041, 0xFE5163,
    0xABDEBB, 0xC561B7, 0x246E3A, 0x424DD2, 0xE00649, 0x2EEA09, 0xD1921C, 0xFE1DEB, 0x1CB129,
    0xA73EE8, 0x8235F5, 0x2EBB44, 0x84E99C, 0x7026B4, 0x5F7E41, 0x3991D6, 0x398353, 0x39F49C,
    0x845F8B, 0xBDF928, 0x3B1FF8, 0x97FFDE, 0x05980F, 0xEF2F11, 0x8B5A0A, 0x6D1F6D, 0x367ECF,
    0x27CB09, 0xB74F46, 0x3F669E, 0x5FEA2D, 0x7527BA, 0xC7EBE5, 0xF17B3D, 0x0739F7, 0x8A5292,
    0xEA6BFB, 0x5FB11F, 0x8D5D08, 0x560330, 0x46FC7B, 0x6BABF0, 0xCFBC20, 0x9AF436, 0x1DA9E3,
    0x91615E, 0xE61B08, 0x659985, 0x5F14A0, 0x68408D, 0xFFD880, 0x4D7327, 0x310606, 0x1556CA,
    0x73A8C9, 0x60E27B, 0xC08C6B,
];

const NPIO2_HW: [i32; 32] = [
    0x3FF921FB, 0x400921FB, 0x4012D97C, 0x401921FB, 0x401F6A7A, 0x4022D97C, 0x4025FDBB, 0x402921FB,
    0x402C463A, 0x402F6A7A, 0x4031475C, 0x4032D97C, 0x40346B9C, 0x4035FDBB, 0x40378FDB, 0x403921FB,
    0x403AB41B, 0x403C463A, 0x403DD85A, 0x403F6A7A, 0x40407E4C, 0x4041475C, 0x4042106C, 0x4042D97C,
    0x4043A28C, 0x40446B9C, 0x404534AC, 0x4045FDBB, 0x4046C6CB, 0x40478FDB, 0x404858EB, 0x404921FB,
];

/// `x` reduced by multiples of π/2: `(n, y0, y1)` with `x - n·π/2 ≈ y0 + y1`.
fn rem_pio2<const F: bool>(x: f64) -> (i32, f64, f64) {
    const INVPIO2: f64 = f64::from_bits(0x3FE45F306DC9C883); // 6.36619772367581382433e-01
    const PIO2_1: f64 = f64::from_bits(0x3FF921FB54400000); // 1.57079632673412561417e+00
    const PIO2_1T: f64 = f64::from_bits(0x3DD0B4611A626331); // 6.07710050650619224932e-11
    const PIO2_2: f64 = f64::from_bits(0x3DD0B4611A600000); // 6.07710050630396597660e-11
    const PIO2_2T: f64 = f64::from_bits(0x3BA3198A2E037073); // 2.02226624879595063154e-21
    const PIO2_3: f64 = f64::from_bits(0x3BA3198A2E000000); // 2.02226624871116645580e-21
    const PIO2_3T: f64 = f64::from_bits(0x397B839A252049C1); // 8.47842766036889956997e-32
    let hx = high_word(x);
    let ix = hx & 0x7FFF_FFFF;
    if ix <= 0x3FE9_21FB {
        return (0, x, 0.0);
    }
    if ix < 0x4002_D97C {
        let s = if hx > 0 { 1.0 } else { -1.0 };
        let mut z = x - s * PIO2_1;
        let (y0, y1) = if ix != 0x3FF9_21FB {
            let y0 = z - s * PIO2_1T;
            (y0, (z - y0) - s * PIO2_1T)
        } else {
            z -= s * PIO2_2;
            let y0 = z - s * PIO2_2T;
            (y0, (z - y0) - s * PIO2_2T)
        };
        return (if hx > 0 { 1 } else { -1 }, y0, y1);
    }
    if ix <= 0x4139_21FB {
        let t = x.abs();
        let m = mad::<F>;
        let n = m(t, INVPIO2, 0.5) as i32;
        let f_n = f64::from(n);
        let mut r = m(-f_n, PIO2_1, t);
        let mut w = f_n * PIO2_1T;
        let quick =
            n < 32 && usize::try_from(n - 1).ok().and_then(|i| NPIO2_HW.get(i)) != Some(&ix);
        let y0 = if quick {
            r - w
        } else {
            let j = ix >> 20;
            let mut y = r - w;
            let i = j - ((high_word(y) >> 20) & 0x7FF);
            if i > 16 {
                let t = r;
                w = f_n * PIO2_2;
                r = t - w;
                w = m(f_n, PIO2_2T, -((t - r) - w));
                y = r - w;
                let i = j - ((high_word(y) >> 20) & 0x7FF);
                if i > 49 {
                    let t = r;
                    w = f_n * PIO2_3;
                    r = t - w;
                    w = m(f_n, PIO2_3T, -((t - r) - w));
                    y = r - w;
                }
            }
            y
        };
        let y1 = (r - y0) - w;
        return if hx < 0 { (-n, -y0, -y1) } else { (n, y0, y1) };
    }
    if ix >= 0x7FF0_0000 {
        return (0, f64::NAN, f64::NAN);
    }
    large_rem_pio2::<F>(x, hx, ix)
}

/// Argument reduction for |x| > 2^19·π/2.
fn large_rem_pio2<const F: bool>(x: f64, hx: i32, ix: i32) -> (i32, f64, f64) {
    const TWO24: f64 = f64::from_bits(0x4170000000000000); // 1.6777216e7
    let e0 = (ix >> 20) - 1046;
    let mut z = with_high_word(
        f64::from_bits(u64::from(low_word(x))),
        ix - ((e0 as u32) << 20) as i32,
    );
    let mut tx = [0.0f64; 3];
    for slot in tx.iter_mut().take(2) {
        *slot = f64::from(z as i32);
        z = (z - *slot) * TWO24;
    }
    if let Some(last) = tx.get_mut(2) {
        *last = z;
    }
    let mut nx = 3;
    while nx > 1 && tx.get(nx - 1).copied() == Some(0.0) {
        nx -= 1;
    }
    let (n, y0, y1) = kernel_rem_pio2::<F>(tx.get(..nx).unwrap_or(&[]), e0);
    if hx < 0 { (-n, -y0, -y1) } else { (n, y0, y1) }
}

fn idx(i: i32) -> usize {
    usize::try_from(i).unwrap_or(usize::MAX)
}

fn get_f(a: &[f64], i: i32) -> f64 {
    a.get(idx(i)).copied().unwrap_or(0.0)
}

fn set_f(a: &mut [f64], i: i32, v: f64) {
    if let Some(s) = a.get_mut(idx(i)) {
        *s = v;
    }
}

fn get_i(a: &[i32], i: i32) -> i32 {
    a.get(idx(i)).copied().unwrap_or(0)
}

fn set_i(a: &mut [i32], i: i32, v: i32) {
    if let Some(s) = a.get_mut(idx(i)) {
        *s = v;
    }
}

const PIO2: [f64; 8] = [
    f64::from_bits(0x3FF921FB40000000), // 1.57079625129699707031e+00
    f64::from_bits(0x3E74442D00000000), // 7.54978941586159635335e-08
    f64::from_bits(0x3CF8469880000000), // 5.39030252995776476554e-15
    f64::from_bits(0x3B78CC5160000000), // 3.28200341580791294123e-22
    f64::from_bits(0x39F01B8380000000), // 1.27065575308067607349e-29
    f64::from_bits(0x387A252040000000), // 1.22933308981111328932e-36
    f64::from_bits(0x36E3822280000000), // 2.73370053816464559624e-44
    f64::from_bits(0x3569F31D00000000), // 2.16741683877804819444e-51
];

/// State of `__kernel_rem_pio2` (precision 2, i.e. two-part result).
struct RemState {
    f: [f64; 20],
    q: [f64; 20],
    iq: [i32; 20],
    jz: i32,
    q0: i32,
}

/// `__kernel_rem_pio2(x, y, e0, nx, prec = 2, two_over_pi)`.
fn kernel_rem_pio2<const F: bool>(x: &[f64], e0: i32) -> (i32, f64, f64) {
    const JK: i32 = 4;
    let jx = i32::try_from(x.len()).unwrap_or(1) - 1;
    let jv = ((e0 - 3) / 24).max(0);
    let mut s = RemState {
        f: [0.0; 20],
        q: [0.0; 20],
        iq: [0; 20],
        jz: JK,
        q0: e0 - 24 * (jv + 1),
    };
    for (i, j) in (0..=(jx + JK)).zip(jv - jx..) {
        let v = if j < 0 {
            0.0
        } else {
            f64::from(get_i(&TWO_OVER_PI, j))
        };
        set_f(&mut s.f, i, v);
    }
    for i in 0..=JK {
        let mut fw = 0.0;
        for j in 0..=jx {
            fw = mad::<F>(get_f(x, j), get_f(&s.f, jx + i - j), fw);
        }
        set_f(&mut s.q, i, fw);
    }
    let (n, ih, z) = loop {
        let (n, ih, z) = distill::<F>(&mut s);
        if z == 0.0 {
            let mut j = 0;
            let mut i = s.jz - 1;
            while i >= JK {
                j |= get_i(&s.iq, i);
                i -= 1;
            }
            if j == 0 {
                let mut k = 1;
                while JK >= k && get_i(&s.iq, JK - k) == 0 {
                    k += 1;
                }
                for i in (s.jz + 1)..=(s.jz + k) {
                    set_f(&mut s.f, jx + i, f64::from(get_i(&TWO_OVER_PI, jv + i)));
                    let mut fw = 0.0;
                    for j in 0..=jx {
                        fw = mad::<F>(get_f(x, j), get_f(&s.f, jx + i - j), fw);
                    }
                    set_f(&mut s.q, i, fw);
                }
                s.jz += k;
                continue;
            }
        }
        break (n, ih, z);
    };
    let (y0, y1) = finish_rem::<F>(&mut s, z, ih);
    (n & 7, y0, y1)
}

/// The `recompute:` block: distils `q` into 24-bit chunks and computes
/// `n`, `ih` and the fractional part `z`.
fn distill<const F: bool>(s: &mut RemState) -> (i32, i32, f64) {
    const TWO24: f64 = f64::from_bits(0x4170000000000000); // 1.6777216e7
    const TWON24: f64 = f64::from_bits(0x3E70000000000000); // 5.9604644775390625e-8
    let jz = s.jz;
    let mut z = get_f(&s.q, jz);
    let (mut i, mut j) = (0, jz);
    while j > 0 {
        let fw = f64::from((TWON24 * z) as i32);
        set_i(&mut s.iq, i, mad::<F>(-TWO24, fw, z) as i32);
        z = get_f(&s.q, j - 1) + fw;
        i += 1;
        j -= 1;
    }
    z = libm::scalbn(z, s.q0);
    z = mad::<F>(-8.0, libm::floor(z * 0.125), z);
    let mut n = z as i32;
    z -= f64::from(n);
    let q0 = s.q0;
    let mut ih = 0;
    if q0 > 0 {
        let top = get_i(&s.iq, jz - 1);
        let i = top >> (24 - q0);
        n += i;
        set_i(&mut s.iq, jz - 1, top - (i << (24 - q0)));
        ih = get_i(&s.iq, jz - 1) >> (23 - q0);
    } else if q0 == 0 {
        ih = get_i(&s.iq, jz - 1) >> 23;
    } else if z >= 0.5 {
        ih = 2;
    }
    if ih > 0 {
        n += 1;
        let mut carry = 0;
        for i in 0..jz {
            let j = get_i(&s.iq, i);
            if carry == 0 {
                if j != 0 {
                    carry = 1;
                    set_i(&mut s.iq, i, 0x0100_0000 - j);
                }
            } else {
                set_i(&mut s.iq, i, 0x00FF_FFFF - j);
            }
        }
        let mask = match q0 {
            1 => 0x7F_FFFF,
            2 => 0x3F_FFFF,
            _ => -1,
        };
        let top = get_i(&s.iq, jz - 1);
        set_i(&mut s.iq, jz - 1, top & mask);
        if ih == 2 {
            z = 1.0 - z;
            if carry != 0 {
                z -= libm::scalbn(1.0, q0);
            }
        }
    }
    (n, ih, z)
}

/// Chops zero terms, converts chunks back to doubles, multiplies by π/2 and
/// compresses the result into two doubles.
fn finish_rem<const F: bool>(s: &mut RemState, mut z: f64, ih: i32) -> (f64, f64) {
    const TWO24: f64 = f64::from_bits(0x4170000000000000); // 1.6777216e7
    const TWON24: f64 = f64::from_bits(0x3E70000000000000); // 5.9604644775390625e-8
    const JP: i32 = 4;
    if z == 0.0 {
        s.jz -= 1;
        s.q0 -= 24;
        while s.jz > 0 && get_i(&s.iq, s.jz) == 0 {
            s.jz -= 1;
            s.q0 -= 24;
        }
    } else {
        z = libm::scalbn(z, -s.q0);
        if z >= TWO24 {
            let fw = f64::from((TWON24 * z) as i32);
            set_i(&mut s.iq, s.jz, mad::<F>(-TWO24, fw, z) as i32);
            s.jz += 1;
            s.q0 += 24;
            set_i(&mut s.iq, s.jz, fw as i32);
        } else {
            set_i(&mut s.iq, s.jz, z as i32);
        }
    }
    let jz = s.jz;
    let mut fw = libm::scalbn(1.0, s.q0);
    let mut i = jz;
    while i >= 0 {
        set_f(&mut s.q, i, fw * f64::from(get_i(&s.iq, i)));
        fw *= TWON24;
        i -= 1;
    }
    let mut fq = [0.0f64; 20];
    let mut i = jz;
    while i >= 0 {
        let mut fw = 0.0;
        let mut k = 0;
        while k <= JP && k <= jz - i {
            fw = mad::<F>(get_f(&PIO2, k), get_f(&s.q, i + k), fw);
            k += 1;
        }
        set_f(&mut fq, jz - i, fw);
        i -= 1;
    }
    let mut fw = 0.0;
    let mut i = jz;
    while i >= 0 {
        fw += get_f(&fq, i);
        i -= 1;
    }
    let y0 = if ih == 0 { fw } else { -fw };
    let mut fw = get_f(&fq, 0) - fw;
    for i in 1..=jz {
        fw += get_f(&fq, i);
    }
    let y1 = if ih == 0 { fw } else { -fw };
    (y0, y1)
}

#[cfg(test)]
mod tests;
