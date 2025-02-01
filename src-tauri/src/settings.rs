// X32 OSC Proxy
// MIT License
//
// Copyright (c) 2025 J.T.Sage
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use std::{fs, path::PathBuf};
use app_dirs2::{app_root, AppDataType, AppInfo};
use serde::{Serialize, Deserialize};

/// APP INFO
const APP_INFO: AppInfo = AppInfo{name: "X32VorAdapter", author: "JTSage.dev"};

// MARK: VorFlag
/// Vor send flag
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize, Default)]
#[serde(into = "bool")]
#[serde(from = "bool")]
pub enum VorFlag {
    /// Send this bundle variant
    Include,
    /// Skip this bundle
    #[default]
    Skip
}

impl VorFlag {
    /// Get the opposite flag
    pub fn switch(self) -> Self {
        match self {
            Self::Include => Self::Skip,
            Self::Skip => Self::Include,
        }
    }
}

impl From<VorFlag> for bool {
    fn from(value: VorFlag) -> Self {
        match value {
            VorFlag::Include => true,
            VorFlag::Skip => false,
        }
    }
}

impl From<bool> for VorFlag {
    fn from(value: bool) -> Self {
        if value { Self::Include } else { Self::Skip }
    }
}

// MARK: AppSettings
/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub struct AppSettings {
    /// X32 IP Address
    pub x32_ip : String,
    /// Vor IP Address
    pub vor_ip : String,
    /// Vor Port
    pub vor_port : String,

    /// Send cue bundle
    #[serde(default)]
    pub send_cue : VorFlag,
    /// Send aux faders
    #[serde(default)]
    pub send_aux : VorFlag,
    /// Send bus faders
    #[serde(default)]
    pub send_bus : VorFlag,
    /// Send DCA faders
    #[serde(default)]
    pub send_dca : VorFlag,
    /// Send channel faders
    #[serde(default)]
    pub send_channel : VorFlag,
    /// Send main faders
    #[serde(default)]
    pub send_main : VorFlag,
    /// Send matrix faders
    #[serde(default)]
    pub send_matrix : VorFlag,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            x32_ip: String::default(),
            vor_ip: String::from("127.0.0.1"),
            vor_port: String::from("3333"),
            send_cue: VorFlag::Include,
            send_aux: VorFlag::Skip,
            send_bus: VorFlag::Skip,
            send_dca: VorFlag::Include,
            send_channel: VorFlag::Skip,
            send_main: VorFlag::Skip,
            send_matrix: VorFlag::Skip
        }
    }
}

// MARK: AppSettings impl
impl AppSettings {
    /// Get path to settings
    fn get_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut path= app_root(AppDataType::UserConfig, &APP_INFO)?;
        path.push("settings.json");
        Ok(path)
    }

    /// Load settings (with defaults)
    pub fn load() -> Self {
        if let Ok(path) = Self::get_path() {
            if let Ok(data) = fs::read_to_string(path.as_path()) {
                if let Ok(settings) = serde_json::from_str(data.as_str()) {
                    // parsed good data, return it
                    return settings
                }
            }
        }

        // if we didn't get a good settings file, go ahead and try and
        // create one from defaults.
        let settings =  Self::default();
        let _ = settings.save();
        settings
    }

    /// Save settings to disk
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_path()?;
        let data = serde_json::to_string_pretty(&self)?;
        fs::write(path.as_path(), data)?;
        Ok(())
    }
}
