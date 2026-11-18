import assert from "node:assert/strict";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import { chromium } from "playwright";
import { freePort } from "./lib/free-port.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
process.env.VITE_DEMO = "1";
let server, browser;
try {
  server = await createServer({ root, server: { host: "127.0.0.1", port: await freePort(), strictPort: true, hmr: false } });
  await server.listen();
  browser = await chromium.launch();
  const page = await browser.newPage();
  await page.addInitScript(() => localStorage.setItem("shieldopt.demo.portable", "1"));
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/settings`, { waitUntil: "networkidle" });
  await page.getByText("Portable edition.", { exact: false }).waitFor();
  assert.equal(await page.locator("#auto-update").count(), 0);
  assert.equal(await page.getByRole("button", { name: /^Update to v/ }).count(), 0);
  await page.getByRole("button", { name: "Check for updates", exact: true }).click();
  await page.getByText(/Download and extract the new portable ZIP|No update found/).waitFor();
  assert.equal(await page.locator("#auto-update").count(), 0);
  assert.equal(await page.getByRole("button", { name: /^Update to v/ }).count(), 0);
  console.log("PASS: portable Settings has manual ZIP updates and no installer controls");
} finally {
  await browser?.close();
  await server?.close();
}
