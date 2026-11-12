import { SHIELD, shieldScenario } from "../lib/worlds.mjs";
import { fileURLToPath } from "node:url";
import { RAM_READ_TIMEOUT_MS } from "../../src/lib/ram-session.ts";

const APP = "com.netflix.ninja";
async function resetRamDevice(ctx) {
  await ctx.reset(shieldScenario());
  await ctx.deviceControl(SHIELD.serial, { user_apps: [APP], texts: {
    "dumpsys activity activities": "ACTIVITY MANAGER ACTIVITIES\nmResumedActivity: ActivityRecord{sim u0 com.google.android.tvlauncher/.MainActivity}",
    "dumpsys activity services": "ACTIVITY MANAGER SERVICES\n",
    "dumpsys media_session": "MEDIA SESSION SERVICE\n",
    "dumpsys meminfo": "Total PSS by process:\n  102,400K: com.netflix.ninja (pid 4242 / activities)\n",
  } });
}

async function prepare(ctx) {
  await resetRamDevice(ctx);
  await ctx.openDevice(SHIELD.key, "ram");
  await ctx.page.getByRole("button", { name: "Refresh RAM", exact: true }).click();
  await ctx.page.getByLabel("Search RAM apps", { exact: true }).fill("Netflix");
  const restrict = ctx.page.getByRole("button", { name: "Restrict Netflix", exact: true });
  await ctx.waitFor(async () => await restrict.isEnabled());
  return restrict;
}

export const scenarios = [
  {
    name: "ram-stop-refresh-and-leave-tab",
    async run(ctx) {
      await resetRamDevice(ctx);
      const page = ctx.page;
      await ctx.openDevice(SHIELD.key, "ram");
      ctx.assert.equal((await ctx.invokes()).filter((call) => call.cmd === "ram_report").length, 0);
      const refresh = page.getByRole("button", { name: "Refresh RAM", exact: true });
      for (const leaveTab of [false, true]) {
        await ctx.fault({ scope: "shell", matches: "cat /proc/meminfo", times: 1, effect: { type: "delay", ms: 30000 } });
        await refresh.click();
        await page.getByRole("button", { name: "Stop RAM refresh", exact: true }).waitFor();
        await ctx.waitFor(async () => (await ctx.state()).faults.some((fault) => fault.fired === 1));
        if (leaveTab) {
          await page.locator("#tab-overview").click();
          await ctx.tab("ram");
        } else {
          const stop = page.getByRole("button", { name: "Stop RAM refresh", exact: true });
          for (const theme of ["dark", "light"]) {
            for (const [width, fontSize] of [[1280, "16px"], [720, "16px"], [720, "32px"]]) {
              await page.setViewportSize({ width, height: 900 });
              await page.evaluate(({ theme, fontSize }) => { document.documentElement.dataset.theme = theme; document.documentElement.style.fontSize = fontSize; }, { theme, fontSize });
              await page.mouse.move(0, 0);
              await stop.focus();
              ctx.assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true);
              ctx.assert.equal(await page.locator(".ram-metrics strong").evaluateAll((values) => values.every((value) => value.scrollWidth <= value.clientWidth)), true);
              await ctx.step(`RAM progress and Stop ${theme} ${width} ${fontSize}`);
            }
          }
          await stop.press("Enter");
          await page.getByRole("status").filter({ hasText: "RAM refresh stopped. No settings were changed." }).waitFor({ timeout: 5000 });
        }
        await ctx.waitFor(async () => await refresh.isEnabled());
        await ctx.clearFaults();
        await refresh.click();
        await page.getByText("100 MB", { exact: true }).waitFor({ timeout: 5000 });
        await ctx.waitFor(async () => await refresh.isEnabled());
        await ctx.step(leaveTab ? "leaving RAM cancels the read and returning stays idle" : "stopped RAM read releases the backend for an immediate retry");
      }
      const calls = await ctx.invokes();
      ctx.assert.equal(calls.filter((call) => call.cmd === "ram_report").length, 4);
      ctx.assert.equal(calls.filter((call) => call.cmd === "ram_cancel_report").length, 2);
      ctx.assert.equal(calls.some((call) => ["ram_restrict", "ram_benchmark_trial"].includes(call.cmd)), false);
      ctx.assert.equal(await page.getByRole("alert").count(), 0);
    },
  },
  {
    name: "ram-manual-read-timeout-and-retry",
    async run(ctx) {
      await resetRamDevice(ctx);
      const page = ctx.page;
      await page.clock.install();
      let held;
      let reads = 0;
      await page.route("**/invoke", async (route) => {
        if (route.request().postDataJSON()?.cmd === "ram_report") {
          reads++;
          if (reads === 1) { held = route; return; }
        }
        await route.continue();
      });
      await ctx.openDevice(SHIELD.key);
      await page.locator("#tab-ram").click();
      const refresh = page.getByRole("button", { name: "Refresh RAM", exact: true });
      ctx.assert.equal(reads, 0);
      ctx.assert.equal(await refresh.isEnabled(), true);
      await ctx.step("opening RAM stays idle until Refresh is clicked");
      await refresh.click();
      await page.getByRole("status").filter({ hasText: "Reading RAM and app safety from the TV. No settings are being changed." }).waitFor();
      await page.getByRole("button", { name: "Stop RAM refresh", exact: true }).waitFor();
      ctx.assert.equal(await refresh.isDisabled(), true);
      await ctx.step("manual RAM read shows progress and Stop");
      await page.clock.fastForward(RAM_READ_TIMEOUT_MS + 1000);
      await page.getByRole("alert").filter({ hasText: "RAM read timed out" }).waitFor();
      ctx.assert.equal(await refresh.isEnabled(), true);
      ctx.assert.equal(reads, 1);
      await ctx.step("a stalled read releases controls for retry");
      await refresh.click();
      await page.getByRole("button", { name: "Restrict Netflix", exact: true }).waitFor();
      await ctx.waitFor(async () => await refresh.isEnabled());
      await page.getByText("100 MB", { exact: true }).waitFor();
      await held.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify({ ok: false, error: "Late response must not replace newer data" }) });
      await page.clock.runFor(50);
      ctx.assert.equal(await page.getByRole("alert").count(), 0);
      ctx.assert.equal(reads, 2);
      ctx.assert.equal((await ctx.invokes()).some((call) => ["ram_restrict", "ram_benchmark_trial"].includes(call.cmd)), false);
      await ctx.step("retry succeeds and ignores the old response");
    },
  },
  {
    name: "ram-automatic-measure-restrict-compare",
    async run(ctx) {
      const restrict = await prepare(ctx), page = ctx.page;
      ctx.assert.equal(await page.getByRole("heading", { name: "Before / after benchmark", exact: true }).count(), 0);
      await page.getByRole("heading", { name: "Apps in RAM", exact: true }).waitFor();
      await page.getByRole("columnheader", { name: "Suggestion", exact: true }).waitFor();
      await page.getByText("100 MB", { exact: true }).waitFor();
      await restrict.click();
      await page.getByText("Automatically measures three launches", { exact: false }).waitFor();
      await ctx.step("one confirmation covers automatic measurements and restriction");
      await page.getByRole("button", { name: "Measure and restrict", exact: true }).click();
      await page.getByText("Optimization and measurements complete.", { exact: true }).waitFor({ timeout: 90000 });
      await page.getByText("Launch-time ranges overlap. No clear speed improvement demonstrated.").waitFor();
      ctx.assert.equal((await ctx.device(SHIELD.serial)).appops[`${APP}|RUN_ANY_IN_BACKGROUND`], "ignore");
      const invokes = (await ctx.invokes()).filter((call) => ["ram_benchmark_trial", "ram_restrict"].includes(call.cmd));
      ctx.assert.deepEqual(invokes.map((call) => call.cmd), ["ram_benchmark_trial", "ram_benchmark_trial", "ram_benchmark_trial", "ram_restrict", "ram_benchmark_trial", "ram_benchmark_trial", "ram_benchmark_trial"]);
      ctx.assert.equal(invokes.filter((call) => call.cmd === "ram_benchmark_trial").every((call) => call.args.returnHome === true), true);
      await page.getByText("Median 100 MB · Peak 100 MB · 3 readings", { exact: true }).waitFor();
      const controls = [page.getByLabel("Benchmark app", { exact: true }), page.getByLabel("Benchmark launch mode", { exact: true }), page.getByRole("button", { name: "Benchmark baseline", exact: true }), page.getByRole("button", { name: "Benchmark comparison", exact: true })];
      const boxes = await Promise.all(controls.map((control) => control.boundingBox()));
      ctx.assert.ok(Math.max(...boxes.map((b) => b.height)) - Math.min(...boxes.map((b) => b.height)) <= 2);
      ctx.assert.ok(Math.max(...boxes.map((b) => b.y)) - Math.min(...boxes.map((b) => b.y)) <= 2);
      for (const theme of ["dark", "light"]) {
        await page.evaluate((value) => document.documentElement.dataset.theme = value, theme);
        await ctx.step(`automatic result and aligned controls ${theme}`);
      }
      await page.getByRole("button", { name: "Undo restriction for Netflix", exact: true }).click();
      await page.getByText("Previous background setting restored and verified.").waitFor();
      ctx.assert.equal((await ctx.device(SHIELD.serial)).appops[`${APP}|RUN_ANY_IN_BACKGROUND`], "allow");
    },
  },
  {
    name: "ram-stop-baseline-without-restricting",
    async run(ctx) {
      const restrict = await prepare(ctx), page = ctx.page;
      await restrict.click();
      await page.getByRole("button", { name: "Measure and restrict", exact: true }).click();
      await page.getByRole("button", { name: "Stop optimization", exact: true }).click();
      await page.getByRole("alert").filter({ hasText: "Baseline incomplete. No restriction was applied." }).first().waitFor({ timeout: 45000 });
      ctx.assert.equal((await ctx.invokes()).some((call) => call.cmd === "ram_restrict"), false);
      ctx.assert.equal(await page.getByRole("heading", { name: "Before / after benchmark", exact: true }).count(), 0);
      await ctx.step("stopping baseline leaves the app unchanged");
    },
  },
  {
    name: "ram-suggestion-and-disabled-reasons",
    async run(ctx) {
      const restrict = await prepare(ctx), page = ctx.page;
      await page.getByRole("checkbox", { name: "Keep Netflix", exact: true }).check();
      await page.getByText("Kept", { exact: true }).waitFor();
      ctx.assert.equal(await restrict.isDisabled(), true);
      const reason = page.getByRole("tooltip", { name: "This app is marked Keep. Uncheck Keep before restricting it.", exact: true });
      await restrict.hover(); await reason.waitFor();
      await restrict.focus(); await restrict.press("Escape"); await reason.waitFor({ state: "hidden" });
      await restrict.press("Enter"); await reason.waitFor();
      ctx.assert.equal(await page.getByRole("region", { name: "Confirm background restriction", exact: true }).count(), 0);
      for (const theme of ["dark", "light"]) {
        for (const [width, fontSize] of [[1280, "16px"], [720, "16px"], [720, "32px"]]) {
          await page.setViewportSize({ width, height: 900 });
          await page.evaluate(({ theme, fontSize }) => { document.documentElement.dataset.theme = theme; document.documentElement.style.fontSize = fontSize; }, { theme, fontSize });
          await page.getByLabel("Search RAM apps", { exact: true }).focus(); await restrict.focus(); await reason.waitFor();
          await ctx.waitFor(async () => { const b = await reason.boundingBox(); return b && b.x >= 0 && b.y >= 0 && b.x + b.width <= width && b.y + b.height <= 900; });
          ctx.assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true);
          await page.screenshot({ path: fileURLToPath(new URL(`../artifacts/ram-suggestion-and-disabled-reasons/viewport-${theme}-${width}-${fontSize}.png`, import.meta.url)) });
          await ctx.step(`suggestions and reasons ${theme} ${width} ${fontSize}`);
        }
      }
      ctx.assert.equal((await ctx.invokes()).some((call) => call.cmd === "ram_restrict"), false);
      await page.getByRole("checkbox", { name: "Keep Netflix", exact: true }).uncheck();
    },
  },
];
