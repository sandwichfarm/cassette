// test.mjs
import { readFile } from 'node:fs/promises';

const wasmBuffer = await readFile('incrementer.wasm');
const wasmModule = await WebAssembly.instantiate(wasmBuffer);
const instance = wasmModule.instance;

console.log('increment(41) =', instance.exports.increment(41));
console.log('add(20, 22) =', instance.exports.add(20, 22));