import { copyFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const v2 = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const assets = resolve(v2, "../assets");
mkdirSync(join(assets, "screenshots"), { recursive: true });
const icon = join(v2, "src-tauri/icons/source-1024.png");
const bytes = readFileSync(icon);
if (bytes.readUInt32BE(16) !== 1024 || bytes.readUInt32BE(20) !== 1024) {
  throw new Error("Expected the original 1024px app icon");
}
copyFileSync(icon, join(assets, "android-tv-optimizer-1024.png"));
copyFileSync(join(v2, "src-tauri/icons/source.svg"), join(assets, "android-tv-optimizer.svg"));
for (const screen of ["overview", "all-apps", "ram-results", "device-display"]) {
  if (!existsSync(join(assets, "screenshots", `${screen}.png`))) {
    throw new Error(`Missing ${screen} screenshot; run npm run screenshots`);
  }
}
console.log("README assets refreshed: vector logo, original 1024px icon and four screenshots.");
