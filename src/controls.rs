use crate::shaders::ShaderSettings;
use minifb::Key;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Controls {
    pub help_visible: bool,
}

impl Controls {
    pub fn handle_key(&mut self, key: Key, settings: &mut ShaderSettings) -> bool {
        match key {
            Key::X => settings.brightness += 0.1,
            Key::Z => settings.brightness -= 0.1,
            Key::E => settings.detail += 0.1,
            Key::Q => settings.detail -= 0.1,
            Key::R => *settings = ShaderSettings::default(),
            Key::H => self.help_visible = !self.help_visible,
            _ => return false,
        }
        *settings = settings.normalized();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brightness_steps_clamp_and_reset_without_affecting_help() {
        let mut controls = Controls::default();
        let mut settings = ShaderSettings::default();
        assert!(controls.handle_key(Key::X, &mut settings));
        assert_eq!(settings.brightness, 1.1);
        assert!(controls.handle_key(Key::Z, &mut settings));
        assert_eq!(settings.brightness, 1.0);
        for _ in 0..30 {
            controls.handle_key(Key::X, &mut settings);
        }
        assert_eq!(settings.brightness, 2.0);
        controls.handle_key(Key::R, &mut settings);
        assert_eq!(settings, ShaderSettings::default());
        assert!(!controls.help_visible);
    }

    #[test]
    fn detail_keys_and_h_toggle_map_to_adjustments() {
        let mut controls = Controls::default();
        let mut settings = ShaderSettings::default();
        controls.handle_key(Key::Q, &mut settings);
        assert!((settings.detail - 0.9).abs() < 1e-6);
        controls.handle_key(Key::E, &mut settings);
        assert_eq!(settings.detail, 1.0);
        for _ in 0..20 {
            controls.handle_key(Key::Q, &mut settings);
        }
        assert_eq!(settings.detail, 0.5);
        assert!(controls.handle_key(Key::H, &mut settings));
        assert!(controls.help_visible);
        assert!(controls.handle_key(Key::H, &mut settings));
        assert!(!controls.help_visible);
    }

    #[test]
    fn old_plus_minus_keys_are_ignored_and_unrelated_keys_are_handled_safely() {
        let mut controls = Controls::default();
        let mut settings = ShaderSettings::default();
        controls.handle_key(Key::NumPadPlus, &mut settings);
        controls.handle_key(Key::NumPadMinus, &mut settings);
        controls.handle_key(Key::Equal, &mut settings);
        controls.handle_key(Key::Minus, &mut settings);
        assert_eq!(settings.brightness, 1.0);
        assert!(!controls.handle_key(Key::Space, &mut settings));
    }
}
