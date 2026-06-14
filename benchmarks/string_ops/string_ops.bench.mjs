export function bench_string_ops() {
    const s = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.";
    let sum = 0;
    let i = 0;
    while (i < 100) {
        if (s.startsWith("Lorem")) sum += 1;
        if (s.endsWith("consequat.")) sum += 2;
        if (s.includes("tempor")) sum += 3;
        const idx = s.indexOf("adipiscing");
        sum += idx;
        const lastIdx = s.lastIndexOf("ut");
        sum += lastIdx;
        i++;
    }
    return sum;
}
