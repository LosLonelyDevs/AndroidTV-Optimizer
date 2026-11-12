import { SHIELD, shieldScenario } from "../lib/worlds.mjs";
import { fileURLToPath } from "node:url";

const APP = "com.netflix.ninja";
async function prepare(ctx) {
  await ctx.reset(shieldScenario());
  await ctx.deviceControl(SHIELD.serial, { user_apps: [APP], texts: {
    "dumpsys activity activities": "ACTIVITY MANAGER ACTIVITIES\nmResumedActivity: ActivityRecord{sim u0 com.google.android.tvlauncher/.MainActivity}",
    "dumpsys activity services": "ACTIVITY MANAGER SERVICES\n",
    "dumpsys media_session": "MEDIA SESSION SERVICE\n",
  } });
  await ctx.openDevice(SHIELD.key, "ram");
  await ctx.page.getByRole("button", { name: "Restrict Netflix", exact: true }).waitFor();
  await ctx.waitFor(async () => await ctx.page.getByRole("button", { name: "Restrict Netflix", exact: true }).isEnabled());
}

export const scenarios = [
  {
    name: "ram-disabled-action-reasons",
    async run(ctx) {
      await prepare(ctx);
      const page = ctx.page;
      const baseline = page.getByRole("button", { name: "Benchmark baseline", exact: true });
      const comparison = page.getByRole("button", { name: "Benchmark comparison", exact: true });
      const reason = page.getByRole("tooltip", { name: "Choose an eligible app from the App menu first.", exact: true });
      ctx.assert.equal(await baseline.isDisabled(), true);
      await baseline.hover();
      await reason.waitFor();
      await page.mouse.move(0, 0);
      await baseline.focus();
      await reason.waitFor();
      await baseline.press("Escape");
      await reason.waitFor({ state: "hidden" });
      await baseline.press("Enter");
      await reason.waitFor();
      ctx.assert.equal(await page.getByRole("region", { name: "Confirm benchmark", exact: true }).count(), 0);
      await comparison.click({ force: true });
      await page.getByRole("tooltip", { name: "Complete all three baseline trials first (0/3 completed).", exact: true }).waitFor();
      ctx.assert.equal(await page.getByRole("region", { name: "Confirm benchmark", exact: true }).count(), 0);
      await ctx.step("unavailable benchmark explains the prerequisite");
      await page.getByRole("checkbox", { name: "Keep Netflix", exact: true }).check();
      const restrict = page.getByRole("button", { name: "Restrict Netflix", exact: true });
      await restrict.click({ force: true });
      await page.getByRole("tooltip", { name: "This app is marked Keep. Uncheck Keep before restricting it.", exact: true }).waitFor();
      ctx.assert.equal(await page.getByRole("region", { name: "Confirm background restriction", exact: true }).count(), 0);
      ctx.assert.equal((await ctx.log()).some((call) => /appops set|am force-stop|am start /.test(call.args.join(" "))), false);
      await ctx.step("kept app remains protected with an accessible reason");
      for (const theme of ["dark", "light"]) {
        for (const [width, fontSize] of [[1280, "16px"], [720, "16px"], [720, "32px"]]) {
          await page.setViewportSize({ width, height: 900 });
          await page.evaluate(({ theme, fontSize }) => {
            document.documentElement.dataset.theme = theme;
            document.documentElement.style.fontSize = fontSize;
          }, { theme, fontSize });
          await page.getByLabel("Search RAM apps", { exact: true }).focus();
          await restrict.focus();
          const tooltip = page.getByRole("tooltip", { name: "This app is marked Keep. Uncheck Keep before restricting it.", exact: true });
          await tooltip.waitFor();
          await ctx.waitFor(async () => {
            const box = await tooltip.boundingBox();
            return box && box.x >= 0 && box.y >= 0 && box.x + box.width <= width && box.y + box.height <= 900;
          });
          ctx.assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true);
          await page.screenshot({ path: fileURLToPath(new URL(`../artifacts/ram-disabled-action-reasons/viewport-${theme}-${width}-${fontSize}.png`, import.meta.url)) });
          await ctx.step(`restriction reason ${theme} ${width} ${fontSize}`);
        }
      }
      await page.getByRole("checkbox", { name: "Keep Netflix", exact: true }).uncheck();
    },
  },
  {
    name: "ram-benchmark-stop-and-unsupported",
    async run(ctx) {
      await prepare(ctx);
      const page = ctx.page;
      await page.getByLabel("Benchmark app", { exact: true }).selectOption(APP);
      await page.getByLabel("Benchmark launch mode", { exact: true }).selectOption({ label: "Cold (force-stop)" });
      await page.getByRole("button", { name: "Benchmark baseline", exact: true }).click();
      await page.getByText("Cold mode force-stops this app before each trial.", { exact: false }).waitFor();
      await page.getByRole("button", { name: "Start three trials", exact: true }).click();
      await page.getByRole("button", { name: "Stop benchmark", exact: true }).click();
      await page.getByText("Benchmark stopped; incomplete runs are not compared.").waitFor({ timeout: 45000 });
      ctx.assert.equal(await page.getByRole("button", { name: "Benchmark comparison", exact: true }).isDisabled(), true);
      const calls = await ctx.log();
      ctx.assert.equal(calls.some((c) => c.args.join(" ").includes("am force-stop")), true);
      await ctx.step("cold benchmark stops without comparing an incomplete run");
      await ctx.fault({ scope: "shell", matches: "cmd appops get", effect: { type: "output", stdout: "Unknown operation RUN_ANY_IN_BACKGROUND\n" } });
      await page.getByRole("button", { name: "Restrict Netflix", exact: true }).click();
      await page.getByRole("button", { name: "Confirm restriction", exact: true }).click();
      await page.getByRole("alert").filter({ hasText: "Background restriction state was not reported" }).waitFor();
      ctx.assert.equal(await page.getByRole("button", { name: "Undo restriction for Netflix", exact: true }).count(), 0);
      await ctx.step("unsupported AppOp fails without changing the app");
    },
  },
  {
    name: "ram-restrict-undo-and-keep",
    async run(ctx) {
      await prepare(ctx);
      const page = ctx.page;
      const restrict = page.getByRole("button", { name: "Restrict Netflix", exact: true });
      const keep = page.getByRole("checkbox", { name: "Keep Netflix", exact: true });
      await keep.check();
      ctx.assert.equal(await restrict.isDisabled(), true);
      await keep.uncheck();
      await restrict.click();
      await page.getByRole("region", { name: "Confirm background restriction" }).waitFor();
      await ctx.step("restriction consequences");
      await page.getByRole("button", { name: "Confirm restriction", exact: true }).click();
      await page.getByText("Restriction setting verified.", { exact: false }).waitFor();
      ctx.assert.equal((await ctx.device(SHIELD.serial)).appops[`${APP}|RUN_ANY_IN_BACKGROUND`], "ignore");
      await ctx.step("verified setting with saved undo");
      await page.getByRole("button", { name: "Undo restriction for Netflix", exact: true }).click();
      await page.getByText("Previous background setting restored and verified.").waitFor();
      ctx.assert.equal((await ctx.device(SHIELD.serial)).appops[`${APP}|RUN_ANY_IN_BACKGROUND`], "allow");
      await ctx.step("undo restores original mode");
    },
  },
  {
    name: "ram-benchmark-before-after",
    async run(ctx) {
      await prepare(ctx);
      const page = ctx.page;
      await page.getByLabel("Benchmark app", { exact: true }).selectOption(APP);
      await page.getByRole("button", { name: "Benchmark baseline", exact: true }).click();
      await page.getByRole("region", { name: "Confirm benchmark" }).waitFor();
      await ctx.step("benchmark interruption confirmation");
      await page.getByRole("button", { name: "Start three trials", exact: true }).click();
      await page.getByText("Baseline (3/3)", { exact: true }).waitFor({ timeout: 90000 });
      await ctx.waitFor(async () => await page.getByRole("button", { name: "Benchmark comparison", exact: true }).isEnabled());
      await page.getByRole("button", { name: "Benchmark comparison", exact: true }).click();
      await page.getByRole("button", { name: "Start three trials", exact: true }).click();
      await page.getByText("Comparison (3/3)", { exact: true }).waitFor({ timeout: 90000 });
      await page.getByText("Launch-time ranges overlap. No clear speed improvement demonstrated.").waitFor();
      await ctx.step("matched trials show zero improvement honestly");
      const calls = await ctx.log();
      ctx.assert.equal(calls.some((c) => c.args.join(" ").includes("am force-stop")), false, "resume benchmark never force-stops");
    },
  },
];
