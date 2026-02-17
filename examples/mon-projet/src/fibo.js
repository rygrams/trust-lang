function fibonacci(n) {
  if (n <= 1) return n;

  let a = 0;
  let b = 1;

  for (let i = 2; i <= n; i++) {
    const next = a + b;
    a = b;
    b = next;
  }

  return b;
}

console.log("=== Fibonacci Demo ===");
const start = performance.now();
const result = fibonacci(50);
const elapsed = performance.now() - start;
console.log(`Result: ${result}`);
console.log(`JS time: ${elapsed.toFixed(3)} ms`);
