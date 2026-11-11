import assert from "node:assert/strict";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import { chromium } from "playwright";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const previousDemo = process.env.VITE_DEMO;
process.env.VITE_DEMO = "1";
let server, browser;
try {
  server = await createServer({ root, server: { host: "127.0.0.1", port: 0, hmr: false } });
  await server.listen();
  browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.addInitScript(() => {
    localStorage.setItem("shieldopt.recentNetworkDevices", JSON.stringify([
      { address: "198.51.100.40:5555", name: "Saved test TV", last_seen_at: Date.now() },
    ]));
  });
  const base = `http://127.0.0.1:${server.httpServer.address().port}`;
  await page.goto(base, { waitUntil: "networkidle" });
  await page.getByRole("button", { name: "Clear Saved test TV from recent devices", exact: true }).click();
  assert.equal(await page.locator('[data-recent-address="198.51.100.40:5555"]').count(), 0);
  assert.equal(await page.evaluate(() => JSON.parse(localStorage.getItem("shieldopt.recentNetworkDevices")).some(item => item.address === "198.51.100.40:5555")), false);

  await page.goto(`${base}/devices/${encodeURIComponent("192.168.1.42:5555")}`, { waitUntil: "networkidle" });
  await page.locator("#tab-apps").click();
  await page.getByRole("button", { name: "All apps", exact: true }).click();
  await page.locator(".kind-chips").waitFor();
  await page.locator(".app-table tbody tr").first().waitFor();
  assert.ok(await page.getByRole("button", { name: /^Curated/ }).count());
  await page.locator("#tab-settings").click();
  for (const section of ["display", "audio", "input", "device", "advanced"]) {
    await page.locator(`#subtab-${section}`).click();
    assert.equal(await page.locator(`#subtab-${section}`).getAttribute("aria-pressed"), "true");
  }
  await page.locator("#tab-tools").click();
  await page.getByRole("heading", { name: "Storage by category", exact: true }).waitFor();
  await page.getByRole("button", { name: "Scan storage", exact: true }).click();
  await page.getByRole("heading", { name: "Largest files", exact: true }).waitFor();
  await page.locator("#tab-sideload").click();
  await page.getByRole("heading", { name: "Quick install", exact: true }).waitFor();
  await page.getByRole("button", { name: "Folder & version manager", exact: true }).click();
  await page.getByRole("heading", { name: "Shizuku", exact: true }).waitFor();
  assert.deepEqual(errors, []);
  console.log("Fork tools passed: recent-address Clear, unified apps, grouped settings, storage, quick install and folder manager.");
} finally {
  await browser?.close();
  await server?.close();
  if (previousDemo === undefined) delete process.env.VITE_DEMO;
  else process.env.VITE_DEMO = previousDemo;
}
