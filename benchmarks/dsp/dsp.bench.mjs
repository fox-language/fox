const N = 1024;
const BIQUAD_PASSES = 64;
const SINE_PASSES = 16;
const RMS_PASSES = 64;
const TWO_PI = 2 * Math.PI;

function fillSignal(buf) {
    for (let i = 0; i < N; i++) {
        const t = i / N;
        buf[i] = 0.5 * Math.sin(TWO_PI * 7.0 * t) + 0.3 * Math.sin(TWO_PI * 41.0 * t);
    }
}

export function bench_dsp_biquad_lowpass() {
    const buf = new Float64Array(N);
    fillSignal(buf);

    const b0 = 0.13110, b1 = 0.26220, b2 = 0.13110;
    const a1 = -0.74783, a2 = 0.27217;

    let checksum = 0;
    for (let pass = 0; pass < BIQUAD_PASSES; pass++) {
        let x1 = 0, x2 = 0, y1 = 0, y2 = 0;
        for (let i = 0; i < N; i++) {
            const x0 = buf[i];
            const y0 = b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
            buf[i] = y0;
            x2 = x1; x1 = x0;
            y2 = y1; y1 = y0;
        }
        checksum += y1;
    }
    return checksum;
}

export function bench_dsp_sine_synth() {
    const buf = new Float64Array(N);
    const phaseInc = 0.05;

    let checksum = 0;
    for (let pass = 0; pass < SINE_PASSES; pass++) {
        let phase = 0;
        for (let i = 0; i < N; i++) {
            buf[i] = Math.sin(phase);
            phase += phaseInc;
            if (phase > TWO_PI) phase -= TWO_PI;
        }
        checksum += buf[0];
    }
    return checksum;
}

export function bench_dsp_rms() {
    const buf = new Float64Array(N);
    fillSignal(buf);

    let total = 0;
    for (let pass = 0; pass < RMS_PASSES; pass++) {
        let sumSq = 0;
        for (let i = 0; i < N; i++) {
            const x = buf[i];
            sumSq += x * x;
        }
        total += Math.sqrt(sumSq / N);
    }
    return total;
}
