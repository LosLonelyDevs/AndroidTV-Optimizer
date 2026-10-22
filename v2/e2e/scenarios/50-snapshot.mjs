// Snapshot: save, preview against an unchanged device (no launcher change,
// real Now values), change the device, preview again, restore, and compare.

import { SHIELD, shieldScenario } from "../lib/worlds.mjs";

const PRINTSPOOLER = "com.android.printspooler";

async function previewNewest(ctx) {
  const page = ctx.page;
  // A previewed snapshot stays selected; reopen the tab for a fresh read.
  await ctx.openDevice(SHIELD.key, "snapshot");
  await page.getByRole("button", { name: "Preview restore" }).first().click();
  await page.locator(".plan-table").waitFor();
  await page.waitForLoadState("networkidle");
}

async function planRow(ctx, item) {
  return ctx.page.locator(".plan-table tr").filter({ hasText: item }).first();
}

export const scenarios = [
  {
    name: "snapshot-save-preview-restore",
    async run(ctx) {
      await ctx.reset(shieldScenario());
      await ctx.shell(SHIELD.serial, "settings put system screen_off_timeout 120000");
      await ctx.shell(SHIELD.serial, "settings put secure location_mode 3; settings put secure doze_enabled 1");
      const original = await ctx.device(SHIELD.serial);
      ctx.assert.ok(original.packages.disabled.includes(PRINTSPOOLER));
      ctx.assert.equal(original.settings.global.window_animation_scale, "0.5");

      await ctx.openDevice(SHIELD.key, "snapshot");
      const page = ctx.page;
      await page.getByRole("button", { name: "Save current state" }).click();
      await page.getByRole("button", { name: "Preview restore" }).first().waitFor();
      await ctx.step("snapshot saved");

      await previewNewest(ctx);
      await page.getByText("0 packages will be disabled", { exact: true }).waitFor();
      ctx.assert.match(await page.locator(".preview-box").innerText(), /0 settings will be written; 0 settings will be reset/);
      const home = page.locator('tr[data-plan-item="launcher"]');
      ctx.assert.equal(await home.count(), 1);
      ctx.assert.match(await home.getAttribute("class"), /plan-noop/, "no launcher change proposed");
      ctx.assert.match(await home.innerText(), /com\.spocky\.projengmenu[\s\S]*Unchanged/);
      const scale = await planRow(ctx, "global.window_animation_scale");
      ctx.assert.match(await scale.innerText(), /0\.5/, "Now shows the device's real value");
      await ctx.step("preview of an unchanged device: no launcher change, real Now values");

      // Change the device behind the app's back.
      await ctx.shell(SHIELD.serial, `pm enable ${PRINTSPOOLER}`);
      await ctx.shell(SHIELD.serial, "settings put global window_animation_scale 1.0");
      await ctx.shell(SHIELD.serial, "settings put system screen_off_timeout 600000");
      await ctx.shell(SHIELD.serial, "settings put secure location_mode 0; settings put secure doze_enabled 0");
      await ctx.shell(SHIELD.serial, "pm enable com.google.android.tvlauncher; cmd package set-home-activity --user 0 com.google.android.tvlauncher/.MainActivity");
      ctx.assert.match((await ctx.device(SHIELD.serial)).home.resolved, /tvlauncher/);

      await previewNewest(ctx);
      ctx.assert.match(await (await planRow(ctx, PRINTSPOOLER)).innerText(), /Enabled[\s\S]*Disabled/);
      ctx.assert.match(await (await planRow(ctx, "global.window_animation_scale")).innerText(), /1\.0[\s\S]*0\.5/);
      ctx.assert.match(await (await planRow(ctx, "system.screen_off_timeout")).innerText(), /600000[\s\S]*120000/);
      const homeChange = page.locator('tr[data-plan-item="launcher"]');
      ctx.assert.doesNotMatch((await homeChange.getAttribute("class")) ?? "", /plan-noop/);
      ctx.assert.match(await homeChange.innerText(), /tvlauncher[\s\S]*com\.spocky\.projengmenu/);
      await ctx.step("preview after changes");

      await page.getByRole("button", { name: "Apply this snapshot" }).click();
      await page.getByRole("button", { name: "Applied", exact: true }).waitFor({ timeout: 60_000 });
      await ctx.step("restored");

      const after = await ctx.device(SHIELD.serial);
      ctx.assert.ok(after.packages.disabled.includes(PRINTSPOOLER), "recorded app disabled again");
      ctx.assert.equal(after.settings.global.window_animation_scale, "0.5", "setting written back");
      ctx.assert.equal(after.settings.system.screen_off_timeout, "120000", "the exact saved timeout is restored");
      ctx.assert.equal(after.settings.secure.location_mode, "3");
      ctx.assert.equal(after.settings.secure.doze_enabled, "1");
      ctx.assert.match(after.home.resolved, /com\.spocky\.projengmenu/, "Home is back on the snapshot's launcher");
      for (const pkg of original.packages.disabled) {
        ctx.assert.ok(after.packages.disabled.includes(pkg), `${pkg} disabled as in the snapshot`);
      }
    },
  },
];
