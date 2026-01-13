use crate::error::Error;
use std::sync::Mutex;
use unicorn_core::{Engine as CoreEngine, EngineAction as CoreEngineAction};

#[derive(uniffi::Enum)]
pub enum EngineAction {
    Reject,
    UpdateComposition { text: String },
    Commit { text: String },
    ShowCandidates { text: String },
}

#[derive(uniffi::Object)]
pub struct Engine {
    inner: Mutex<CoreEngine>,
}

#[uniffi::export]
impl Engine {
    #[uniffi::constructor]
    pub fn new(json_data: String) -> Result<Self, Error> {
        match CoreEngine::new(&json_data) {
            Ok(engine) => Ok(Self {
                inner: Mutex::new(engine),
            }),
            Err(e) => Err(Error::Init {
                message: e.to_string(),
            }),
        }
    }

    #[uniffi::constructor]
    pub fn new_from_path(path: String) -> Result<Self, Error> {
        let json_data = std::fs::read_to_string(path).map_err(|e| Error::Init {
            message: e.to_string(),
        })?;
        Self::new(json_data)
    }

    pub fn process_key(&self, char_code: u32) -> Vec<EngineAction> {
        let mut engine = self.inner.lock().unwrap();
        if let Some(c) = std::char::from_u32(char_code) {
            engine
                .process_key(c)
                .into_iter()
                .map(|action| match action {
                    CoreEngineAction::Reject => EngineAction::Reject,
                    CoreEngineAction::UpdateComposition(text) => {
                        EngineAction::UpdateComposition { text }
                    }
                    CoreEngineAction::Commit(text) => EngineAction::Commit { text },
                    CoreEngineAction::ShowCandidates(text) => EngineAction::ShowCandidates { text },
                })
                .collect()
        } else {
            vec![EngineAction::Reject]
        }
    }

    pub fn get_candidates(&self) -> Vec<String> {
        let engine = self.inner.lock().unwrap();
        engine.get_candidates()
    }

    pub fn select_candidate(&self, index: u32) {
        let mut engine = self.inner.lock().unwrap();
        engine.select_candidate(index as usize);
    }

    pub fn deactivate(&self) {
        let mut engine = self.inner.lock().unwrap();
        engine.deactivate();
    }
}
