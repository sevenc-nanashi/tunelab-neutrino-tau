#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct NeutrinoInfo {
    speaker: NeutrinoSpeakerInfo,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct NeutrinoSpeakerInfo {
    name: String,
    gender: String,
    language: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VoiceSource {
    id: String,
    name: String,
    description: String,
}

impl VoiceSource {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let info_path = path.join("info.toml");
        if info_path.exists() {
            let info_str = std::fs::read_to_string(&info_path)
                .map_err(|e| anyhow::anyhow!("Failed to read info.toml: {}", e))?;
            let info: NeutrinoInfo = toml::from_str(&info_str)
                .map_err(|e| anyhow::anyhow!("Failed to parse info.toml: {}", e))?;

            Ok(Self {
                id: path.file_name().unwrap().to_string_lossy().to_string(),
                name: info.speaker.name,
                description: format!(
                    "Gender={}, Language={}",
                    info.speaker.gender, info.speaker.language
                ),
            })
        } else {
            Ok(Self {
                id: path.file_name().unwrap().to_string_lossy().to_string(),
                name: path.file_name().unwrap().to_string_lossy().to_string(),
                description: "No info.toml found".to_string(),
            })
        }
    }
}
