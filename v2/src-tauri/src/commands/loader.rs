pub use shield_optimizer_core::commands::loader::*;

pub fn load_quick_apps() -> Vec<super::quick_install::QuickApp> {
    match serde_json::from_str(include_str!(
        "../../../crates/core/data/app-lists/quick-install.json"
    )) {
        Ok(apps) => apps,
        Err(e) => {
            tracing::error!(error = %e, "quick-install.json parse error; using empty list");
            Vec::new()
        }
    }
}
