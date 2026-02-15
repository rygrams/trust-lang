function main() {
  const s = "abcdef";
  const minusOne = 0 - 1;
  const minusTwo = 0 - 2;

  console.log(`slice(-2): ${s.slice(minusTwo)}`);
  console.log(`slice(1, -1): ${s.slice(1, minusOne)}`);
  console.log(`substring(-2, 2): ${s.substring(minusTwo, 2)}`);
  console.log(`substring(4, 2): ${s.substring(4, 2)}`);
  console.log(`substr(-2, 2): ${s.substr(minusTwo, 2)}`);
  console.log(`charAt(-1): ${s.charAt(minusOne)}`);
  console.log(`at(-1): ${s.at(minusOne)}`);
  console.log(`charAt(99): ${s.charAt(99)}`);
  console.log(`at(99): ${s.at(99)}`);

  const u = "👍a";
  console.log(`unicode length (u.length): ${String(u.length)}`);
  console.log(`unicode indexOf(a): ${String(u.indexOf("a"))}`);
  console.log(`unicode at(-1): ${u.at(minusOne)}`);
}

main();
