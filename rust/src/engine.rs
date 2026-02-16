use crate::config;

#[derive(Debug)]
pub struct Engine {
    neutrino_path: std::path::PathBuf,
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

        Ok(Self {
            neutrino_path: config.neutrino_path.unwrap().into(),
        })
    }

    fn load_voices(&self) -> anyhow::Result<Vec<crate::speaker::VoiceSource>> {
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
}
