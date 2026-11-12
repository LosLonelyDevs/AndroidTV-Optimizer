import { SHIELD, shieldScenario } from "../lib/worlds.mjs";
import { fileURLToPath } from "node:url";

const APP = "com.netflix.ninja";
async function prepare(ctx) {
  await ctx.reset(shieldScenario());
  await ctx.deviceControl(SHIELD.serial, { user_apps: [APP], texts: {
    "dumpsys activity activities": "ACTIVITY MANAGER ACTIVITIES\nmResumedActivity: ActivityRecord{sim u0 com.google.android.tvlauncher/.MainActivity}",
    "dumpsys activity services": "ACTIVITY MANAGER SERVICES\n",
    "dumpsys media_session": "MEDIA SESSION SERVICE\n",
    "dumpsys meminfo": "Total PSS by process:\n  102,400K: com.netflix.ninja (pid 4242 / activities)\n",
  } });
  await ctx.openDevice(SHIELD.key, "ram");
  await ctx.page.getByLabel("Search RAM apps", { exact: true }).fill("Netflix");
  const restrict = ctx.page.getByRole("button", { name: "Restrict Netflix", exact: true });
  await ctx.waitFor(async () => await restrict.isEnabled());
  return restrict;
}

export const scenarios = [
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
