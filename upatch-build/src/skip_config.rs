use std::collections::HashSet;
use std::fs;
use std::path::Path;
use log::info;
use anyhow::{bail, Result};

#[derive(Debug, Clone)]
pub struct SkipConfig {
    skip_patterns: HashSet<String>,
}

impl SkipConfig {
    pub fn new() -> Self {
        Self {
            skip_patterns: HashSet::new(),
        }
    }

    pub fn from_args(skip_file_path: Option<&Path>) -> Result<Self, anyhow::Error> {
        let mut config = Self::new();
        
        if let Some(file_path) = skip_file_path {
            config.load_from_file(file_path)?;
        }
        
        Ok(config)
    }

    fn load_from_file(&mut self, file_path: &Path) -> Result<(), anyhow::Error> {
        if !file_path.exists() {
            bail!("Configuration file not found: {}", file_path.display());
        }
        
        let content = fs::read_to_string(file_path)?;
        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                self.skip_patterns.insert(line.to_string());
            }
        }
        
        Ok(())
    }

    pub fn should_skip(&self, object_path: &Path) -> bool {
        if let Some(file_name) = object_path.file_name() {
            let file_name_str = file_name.to_string_lossy();
    
            for pattern in &self.skip_patterns {
                if file_name_str.contains(pattern) {
                    info!(
                        "Skipping object '{}' because it matches skip pattern: {}",
                        file_name_str, pattern
                    );
                    return true;
                }
            }
            false
        } else {
            false
        }
    }

    pub fn get_skip_count(&self) -> usize {
        self.skip_patterns.len()
    }
}