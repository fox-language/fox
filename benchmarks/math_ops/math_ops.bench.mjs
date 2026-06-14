export function bench_math_complex() {
    let sum = 0;
    let i = 1;
    while (i <= 1000) {
        sum += Math.sin(i) * Math.cos(i) + Math.sqrt(i) + Math.log(i);
        i++;
    }
    return sum;
}
