import sharp from "sharp";
import { readFileSync } from "node:fs";

const svg = readFileSync(new URL("../assets/tray.svg", import.meta.url), "utf8");
const white = svg.replace('fill="#000000"', 'fill="#ffffff"');
const out = (n) => new URL(`../src-tauri/icons/${n}`, import.meta.url).pathname;

await sharp(Buffer.from(svg)).resize(22, 22).png().toFile(out("tray-22.png"));
await sharp(Buffer.from(svg)).resize(44, 44).png().toFile(out("tray-44.png"));
await sharp(Buffer.from(white)).resize(32, 32).png().toFile(out("tray-win-32.png"));
console.log("tray icons written");
