export function bench_vec_ops() {
    const v = [];
    let i = 0;
    while (i < 10000) {
        v.push(i);
        i++;
    }
    let sum = 0;
    i = 0;
    while (i < 10000) {
        const x = v[i];
        if (x !== undefined) {
            v[i] = x * 2;
            sum += x;
        }
        i++;
    }
    i = 0;
    while (i < 5000) {
        const x = v.pop();
        if (x !== undefined) {
            sum -= x;
        }
        i++;
    }
    return sum + v.length;
}
