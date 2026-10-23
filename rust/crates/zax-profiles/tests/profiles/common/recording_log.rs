use std::cell::RefCell;

use zax_profiles::format::ProfileLog;

#[derive(Default)]
pub struct RecordingProfileLog {
    pub informations: RefCell<Vec<String>>,
    pub warnings: RefCell<Vec<String>>,
    pub errors: RefCell<Vec<String>>,
}

impl RecordingProfileLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn any_contains(&self, fragment: &str) -> bool {
        [&self.informations, &self.warnings, &self.errors]
            .iter()
            .any(|lines| lines.borrow().iter().any(|line| line.contains(fragment)))
    }
}

impl ProfileLog for RecordingProfileLog {
    fn information(&self, message: &str) {
        self.informations.borrow_mut().push(message.to_owned());
    }

    fn warning(&self, message: &str) {
        self.warnings.borrow_mut().push(message.to_owned());
    }

    fn error(&self, message: &str) {
        self.errors.borrow_mut().push(message.to_owned());
    }
}
