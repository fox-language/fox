export function bench_closure_ops() {
    let sum = 0;
    let i = 0;
    while (i < 10000) {
        let a = i;
        let b = i * 2;
        let c = i * 3;
        let compute = (x) => {
            return (x * a) - b + c;
        };
        sum = (sum + compute(i)) % 99991;
        i = i + 1;
    }
    return sum;
}
