const N: usize = 1024;
const BIQUAD_PASSES: i32 = 64;
const SINE_PASSES: i32 = 16;
const RMS_PASSES: i32 = 64;
const TWO_PI: f64 = 2.0 * std::f64::consts::PI;

fn fill_signal(buf: &mut [f64; N]) {
    for i in 0..N {
        let t = i as f64 / N as f64;
        buf[i] = 0.5 * (TWO_PI * 7.0 * t).sin() + 0.3 * (TWO_PI * 41.0 * t).sin();
    }
}

#[no_mangle]
pub extern "C" fn bench_dsp_biquad_lowpass() -> f64 {
    let mut buf: [f64; N] = [0.0; N];
    fill_signal(&mut buf);

    let b0: f64 = 0.13110;
    let b1: f64 = 0.26220;
    let b2: f64 = 0.13110;
    let a1: f64 = -0.74783;
    let a2: f64 = 0.27217;

    let mut checksum = 0.0;
    for _ in 0..BIQUAD_PASSES {
        let mut x1 = 0.0;
        let mut x2 = 0.0;
        let mut y1 = 0.0;
        let mut y2 = 0.0;
        for i in 0..N {
            let x0 = buf[i];
            let y0 = b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
            buf[i] = y0;
            x2 = x1;
            x1 = x0;
            y2 = y1;
            y1 = y0;
        }
        checksum += y1;
    }
    checksum
}

#[no_mangle]
pub extern "C" fn bench_dsp_sine_synth() -> f64 {
    let mut buf: [f64; N] = [0.0; N];
    let phase_inc: f64 = 0.05;

    let mut checksum = 0.0;
    for _ in 0..SINE_PASSES {
        let mut phase: f64 = 0.0;
        for i in 0..N {
            buf[i] = phase.sin();
            phase += phase_inc;
            if phase > TWO_PI {
                phase -= TWO_PI;
            }
        }
        checksum += buf[0];
    }
    checksum
}

#[no_mangle]
pub extern "C" fn bench_dsp_rms() -> f64 {
    let mut buf: [f64; N] = [0.0; N];
    fill_signal(&mut buf);

    let mut total = 0.0;
    for _ in 0..RMS_PASSES {
        let mut sum_sq = 0.0;
        for i in 0..N {
            let x = buf[i];
            sum_sq += x * x;
        }
        total += (sum_sq / N as f64).sqrt();
    }
    total
}
