use std::{env, fs::File, path::Path};

use bevy::prelude::{debug, error, KeyCode, MouseButton};
use kurinji::{EventPhase, Kurinji};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct CobbleConfig {
    pub video: VideoConfig,
    pub debug: DebugConfig,
    pub game: GameConfig,
    pub input: InputConfig,
}

impl CobbleConfig {
    pub fn default_as_yaml() -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(&CobbleConfig::default())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct DebugConfig {
    pub print_default_config: bool,
    pub show_colliders: bool,
    pub show_fps: bool,
    pub show_selection: bool,
    pub show_selection_normal: bool,
    pub log_diagnostics: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct VideoConfig {
    pub msaa_samples: u32,
    pub show_interface: bool,
    pub vsync: bool,
    pub window_mode: WindowMode,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct GameConfig {
    pub creative: bool,
    pub breakable_bedrock: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct InputConfig {
    pub bindings: kurinji::Bindings,
    pub sensitivity: f32,
    pub initial_cursor_grab: bool,
}

impl VideoConfig {
    pub fn to_window_mode(&self) -> bevy::window::WindowMode {
        match self.window_mode {
            WindowMode::Windowed => bevy::window::WindowMode::Windowed,
            WindowMode::Borderless => bevy::window::WindowMode::BorderlessFullscreen,
            WindowMode::Fullscreen => bevy::window::WindowMode::Fullscreen { use_size: false },
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum WindowMode {
    Windowed,
    Borderless,
    Fullscreen,
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            msaa_samples: 4,
            show_interface: true,
            vsync: true,
            window_mode: WindowMode::Windowed,
        }
    }
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            print_default_config: false,
            show_colliders: false,
            show_fps: true,
            show_selection: true,
            show_selection_normal: false,
            log_diagnostics: false,
        }
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            creative: true,
            breakable_bedrock: false,
        }
    }
}

impl Default for InputConfig {
    fn default() -> Self {
        let bindings = Kurinji::default()
            .bind_keyboard_pressed(KeyCode::Key1, "SLOT_1")
            .bind_keyboard_pressed(KeyCode::Key2, "SLOT_2")
            .bind_keyboard_pressed(KeyCode::Key3, "SLOT_3")
            .bind_keyboard_pressed(KeyCode::Key4, "SLOT_4")
            .bind_keyboard_pressed(KeyCode::Key5, "SLOT_5")
            .bind_keyboard_pressed(KeyCode::Key6, "SLOT_6")
            .bind_keyboard_pressed(KeyCode::Key7, "SLOT_7")
            .bind_keyboard_pressed(KeyCode::Key8, "SLOT_8")
            .bind_keyboard_pressed(KeyCode::Key9, "SLOT_9")
            .bind_keyboard_pressed(KeyCode::Escape, "PAUSE")
            .set_event_phase("PAUSE", EventPhase::OnBegin)
            .bind_keyboard_pressed(KeyCode::Tab, "FLY_TOGGLE")
            .bind_keyboard_pressed(KeyCode::W, "MOVE_FORWARD")
            .bind_keyboard_pressed(KeyCode::S, "MOVE_BACKWARD")
            .bind_keyboard_pressed(KeyCode::A, "MOVE_LEFT")
            .bind_keyboard_pressed(KeyCode::D, "MOVE_RIGHT")
            .bind_keyboard_pressed(KeyCode::Space, "MOVE_JUMP")
            .bind_keyboard_pressed(KeyCode::LShift, "MOVE_MOD_SLOW_DESC")
            .bind_keyboard_pressed(KeyCode::LControl, "MOVE_MOD_FAST")
            .bind_mouse_button_pressed(MouseButton::Middle, "PICK_BLOCK")
            .bind_mouse_button_pressed(MouseButton::Left, "BREAK")
            .set_event_phase("BREAK", EventPhase::OnBegin)
            .bind_mouse_button_pressed(MouseButton::Right, "PLACE")
            .set_event_phase("PLACE", EventPhase::OnBegin)
            .get_bindings();
        Self {
            bindings,
            sensitivity: 1.0,
            initial_cursor_grab: cfg!(not(target_arch = "wasm")),
        }
    }
}

/// Try loading the config by trying the local file first and then the global in
/// XDG_CONFIG_HOME
fn open_config() -> Option<File> {
    let local_path = Path::new("./cobble.yaml");
    if let Ok(f) = File::open(local_path) {
        debug!("Local config file found");
        return Some(f);
    }
    let config_path = Path::new(
        &(match env::var("XDG_CONFIG_HOME") {
            Ok(f) => f,
            Err(_) => "~/.config/".to_owned(),
        }),
    )
    .with_file_name("cobble.yaml");
    File::open(config_path).ok()
}

pub fn load() -> CobbleConfig {
    open_config().map_or_else(
        CobbleConfig::default,
        |reader| match serde_yaml::from_reader(reader) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to parse config file: {}", e);
                CobbleConfig::default()
            }
        },
    )
}
