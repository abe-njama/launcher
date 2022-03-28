use log;
use std::{collections::HashMap, path::PathBuf};
use tauri::api::process::{Command, CommandEvent};

use lair_keystore_manager::error::LaunchTauriSidecarError;

use super::HolochainVersion;

pub fn launch_holochain_process(
  log_level: log::Level,
  holochain_version: HolochainVersion,
  conductor_config_path: PathBuf,
) -> Result<(), LaunchTauriSidecarError> {
  let mut envs = HashMap::new();
  envs.insert(String::from("RUST_LOG"), String::from(log_level.as_str()));

  let (mut holochain_rx, holochain_child) = Command::new_sidecar("holochain") // TODO: Fix
    .or(Err(LaunchTauriSidecarError::BinaryNotFound))?
    .args(&[
      "-c",
      conductor_config_path.into_os_string().to_str().unwrap(),
    ])
    .envs(envs)
    .spawn()
    .map_err(|err| LaunchTauriSidecarError::FailedToExecute(format!("{}", err)))?;

  tauri::async_runtime::spawn(async move {
    // read events such as stdout
    while let Some(event) = holochain_rx.recv().await {
      match event.clone() {
        CommandEvent::Stdout(line) => log::info!("[HOLOCHAIN] {}", line),
        CommandEvent::Stderr(line) => log::info!("[HOLOCHAIN] {}", line),
        _ => log::info!("[HOLOCHAIN] {:?}", event),
      };
      if format!("{:?}", event).contains("Installing lair_keystore") {
        // Lair keystore can't be executed, Holochain is trying to download and install Lair, kill it
        log::error!("Holochain is trying to download and install lair_keystore directly! Killing Holochain...");
        let result = holochain_child.kill();
        log::error!("Holochain terminated: {:?}", result);
        break;
      }
    }
  });
  log::info!("Launched holochain");

  Ok(())
}