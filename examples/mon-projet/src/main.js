function fibonacci(n) {
    if (n <= 1) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

function main() {
    const inputs = [0, 1, 5, 40];

    for (const n of inputs) {
        const start = performance.now();
        const result = fibonacci(n);
        const elapsed = performance.now() - start;
        console.log(`fibonacci(${n}) = ${result} [${elapsed.toFixed(3)}ms]`);
    }
}

main();
