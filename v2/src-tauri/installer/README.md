# Windows EXE maintenance

`installer.nsi` is Tauri CLI 2.11.2's stock NSIS template, with the MSI migration
choice corrected. Upstream source:
https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.2/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi

The upstream callback unconditionally uninstalls an existing MSI, even when the
user selects "Do not uninstall". Our MSI maintenance page explains the conversion
and makes that choice close setup with Windows cancellation code 1602. It never
launches the MSI uninstaller or installs EXE-owned files over an MSI-owned copy.
Use the MSI installer for an in-place MSI update. Choosing the first option still
explicitly permits conversion; passive and existing NSIS paths retain
the stock behavior.

The MSI page does not compare its encoded Windows Installer version with the
app's semantic version in its displayed copy. Keeping the MSI installation stays
available even when downgrades are disabled.

When updating Tauri CLI, compare its template with this copy and carry these
maintenance-page changes forward. Preserve the bundled Tauri MIT license.
