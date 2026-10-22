import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

assert.equal(process.platform, "win32", "this regression runs the Windows NSIS engine");
const root = dirname(dirname(fileURLToPath(import.meta.url)));
const config = JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8"));
const template = readFileSync(join(root, "src-tauri", config.bundle.windows.nsis.template), "utf8");
const callback = template.match(/Function PageLeaveReinstall\r?\n([\s\S]*?)\r?\nFunctionEnd/)[1];
assert.equal((callback.match(/\$\{NSD_GetState\} \$R2 \$R1/g) ?? []).length, 1);
assert.match(template, /Do not uninstall \(close setup\)/);
assert.match(template, /close this setup and run the new MSI installer/);
const compiler = [
  process.env.NSIS_COMPILER,
  process.env["ProgramFiles(x86)"] && join(process.env["ProgramFiles(x86)"], "NSIS/makensis.exe"),
  process.env.LOCALAPPDATA && join(process.env.LOCALAPPDATA, "tauri/NSIS/makensis.exe"),
].find((p) => p && existsSync(p));
assert.ok(compiler, "install NSIS or set NSIS_COMPILER");

// Execute the production callback, replacing only its UI input and side effects.
const decision = callback.split("  reinst_uninstall:")[0]
  .replace("${NSD_GetState} $R2 $R1", "StrCpy $R1 ${SELECTED}");
const cases = [
  ...[-1, 0, 1].flatMap((comparison) => [
    { name: `MSI keep ${comparison}`, wix: 1, comparison, selected: 0, expected: 1602 },
    { name: `MSI replace ${comparison}`, wix: 1, comparison, selected: 1, expected: 42 },
  ]),
  { name: "MSI keep from interactive updater", wix: 1, update: 1, selected: 0, expected: 1602 },
  { name: "MSI passive conversion unchanged", wix: 1, passive: 1, selected: 0, expected: 42 },
  { name: "NSIS reinstall", comparison: 0, selected: 1, expected: 0 },
  { name: "NSIS remove", comparison: 0, selected: 0, expected: 42 },
  { name: "NSIS upgrade without uninstall", comparison: 1, selected: 0, expected: 0 },
  { name: "NSIS downgrade without uninstall", comparison: -1, selected: 0, expected: 0 },
  { name: "NSIS upgrade with uninstall", comparison: 1, selected: 1, expected: 42 },
  { name: "NSIS updater", comparison: 1, update: 1, selected: 1, expected: 0 },
];
const work = mkdtempSync(join(tmpdir(), "shield-installer-test-"));
try {
  for (const [index, scenario] of cases.entries()) {
    const filename = join(work, `case-${index}.nsi`);
    const executable = join(work, `case-${index}.exe`);
    writeFileSync(filename, `Unicode true
!include LogicLib.nsh
!include nsDialogs.nsh
!define SELECTED ${scenario.selected}
Name "Installer maintenance regression"
OutFile "${executable}"
RequestExecutionLevel user
SilentInstall silent
Var WixMode
Var PassiveMode
Var UpdateMode
Section
  StrCpy $WixMode ${scenario.wix ?? 0}
  StrCpy $PassiveMode ${scenario.passive ?? 0}
  StrCpy $UpdateMode ${scenario.update ?? 0}
  StrCpy $R0 ${scenario.comparison ?? 1}
  Call PageLeaveReinstall
  SetErrorLevel 99
SectionEnd
Function PageLeaveReinstall
${decision}
  reinst_uninstall:
    SetErrorLevel 42
    Quit
  reinst_done:
    SetErrorLevel 0
    Quit
FunctionEnd
`);
    const build = spawnSync(compiler, ["/V2", filename], { encoding: "utf8", windowsHide: true, timeout: 30000 });
    assert.equal(build.status, 0, `${scenario.name}: ${build.error ?? ""}${build.stdout}${build.stderr}`);
    const result = spawnSync(executable, [], { windowsHide: true, timeout: 10000 });
    assert.equal(result.status, scenario.expected, `${scenario.name}: ${result.error ?? "unexpected installer decision"}`);
  }
  console.log(`Installer maintenance passed: ${cases.length} compiled NSIS scenarios; declining MSI conversion never reaches the uninstaller.`);
} finally {
  assert.ok(resolve(work).startsWith(resolve(tmpdir()) + "\\"));
  rmSync(work, { recursive: true, force: true });
}
