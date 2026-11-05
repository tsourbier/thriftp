pub mod tabs;

use crate::model::Preference;

#[derive(Default)]
pub struct PreferencesService {
    preference: Preference,
}

impl PreferencesService {
    pub fn get(&self) -> Preference {
        self.preference.clone()
    }

    pub fn set(&mut self, preference: Preference) {
        self.preference = preference;
    }
}
