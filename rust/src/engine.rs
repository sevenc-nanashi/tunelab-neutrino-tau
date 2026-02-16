use crate::config;
use std::os::windows::process::CommandExt;

#[derive(Debug)]
pub struct Engine {
    neutrino_path: std::path::PathBuf,
    server: Option<std::process::Child>,
}

impl Engine {
    pub fn new(dll_path: std::path::PathBuf) -> anyhow::Result<Self> {
        let config_path = dll_path.join("config.json");
        let mut config = if config_path.exists() {
            let config_str = std::fs::read_to_string(&config_path)
                .map_err(|e| anyhow::anyhow!("Failed to read config file: {}", e))?;
            serde_json::from_str(&config_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse config file: {}", e))?
        } else {
            config::Config::default()
        };
        if config.neutrino_path.is_none() {
            if let Some(result) = native_dialog::FileDialogBuilder::default()
                .open_single_dir()
                .show()?
            {
                let path = result.to_string_lossy().to_string();
                println!("Selected Neutrino path: {}", path);
                config.neutrino_path = Some(path);

                std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)
                    .map_err(|e| anyhow::anyhow!("Failed to write config file: {}", e))?;
            } else {
                return Err(anyhow::anyhow!(
                    "Neutrino path is required but not provided"
                ));
            }
        }

        let neutrino_path = config.neutrino_path.as_ref().unwrap();
        if !std::path::Path::new(neutrino_path).exists() {
            return Err(anyhow::anyhow!(
                "Neutrino path does not exist: {}",
                neutrino_path
            ));
        }
        let server_path = std::path::Path::new(neutrino_path)
            .join("bin")
            .join("neutrino_server.exe");
        if !server_path.exists() {
            return Err(anyhow::anyhow!(
                "Neutrino server executable not found at: {}",
                server_path.display()
            ));
        }

        let server = std::process::Command::new(server_path)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .map_err(|e| anyhow::anyhow!("Failed to start Neutrino server: {}", e))?;

        Ok(Self {
            neutrino_path: config.neutrino_path.unwrap().into(),
            server: Some(server),
        })
    }

    pub fn load_voices(&self) -> anyhow::Result<Vec<crate::speaker::VoiceSource>> {
        let mut speakers = Vec::new();
        let models_path = self.neutrino_path.join("model");
        if !models_path.exists() {
            return Err(anyhow::anyhow!("Neutrino model directory not found"));
        }

        for entry in std::fs::read_dir(models_path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                match crate::speaker::VoiceSource::load(&entry.path()) {
                    Ok(voice) => speakers.push(voice),
                    Err(e) => eprintln!(
                        "Failed to load voice from {}: {}",
                        entry.path().display(),
                        e
                    ),
                }
            }
        }

        Ok(speakers)
    }

    pub fn synthesize(&self, synthesis_task_json: &str) -> anyhow::Result<String> {
        crate::synthesize_task::synthesize_task_json(synthesis_task_json)
            .map_err(|e| anyhow::anyhow!(e))
    }

    fn invoke_client(&self, args: &[&str]) -> anyhow::Result<String> {
        let client_path = std::path::Path::new(&self.neutrino_path)
            .join("bin")
            .join("neutrino_client.exe");
        if !client_path.exists() {
            return Err(anyhow::anyhow!(
                "Neutrino client executable not found at: {}",
                client_path.display()
            ));
        }

        let output = std::process::Command::new(client_path)
            .args(args)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to execute Neutrino client: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(anyhow::anyhow!(
                "Neutrino client error: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    pub fn shutdown(&mut self) {
        if let Ok(status) = self.invoke_client(&["shutdown"]) {
            println!("Neutrino server shutdown response: {}", status);
        } else {
            eprintln!("Failed to send shutdown command to Neutrino server");

            if let Err(e) = self.server.as_mut().unwrap().kill() {
                eprintln!("Failed to kill Neutrino server process: {}", e);
            } else {
                println!("Neutrino server process killed successfully");
            }
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.shutdown();
    }
}
