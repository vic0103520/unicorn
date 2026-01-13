use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Deserialize, Clone, Debug)]
pub struct TrieNode {
    #[serde(rename = ">>")]
    pub candidates: Option<Vec<String>>,
    #[serde(flatten)]
    pub children: HashMap<String, TrieNode>,
}

#[derive(Debug, PartialEq)]
/// Represents the action the frontend should take in response to a key event.
pub enum EngineAction {
    /// The input key was invalid for the current state.
    /// The frontend should typically ignore this event or provide feedback (e.g., beep).
    Reject,

    /// The composition buffer has changed.
    /// The frontend should update the inline pre-edit text with the provided string.
    UpdateComposition(String),

    /// The composition is finished or a symbol has been selected.
    /// The frontend should insert the provided string into the target application
    /// and clear the composition state.
    Commit(String),

    /// The engine has found multiple candidates for the current input.
    /// The frontend should display a candidate window allowing the user to select one.
    /// The provided string is the current composition text.
    ShowCandidates(String),
}

pub struct Engine {
    root: Arc<TrieNode>,
    path: Vec<Arc<TrieNode>>,
    buffer: String,
    active: bool,
    selected_candidate: usize,
}

impl Engine {
    pub fn new(json_data: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let root: TrieNode = serde_json::from_str(json_data)?;
        let root = Arc::new(root);
        Ok(Self {
            path: vec![Arc::clone(&root)],
            root,
            buffer: String::new(),
            active: false,
            selected_candidate: 0,
        })
    }

    fn current_node(&self) -> Option<&Arc<TrieNode>> {
        self.path.last()
    }

    pub fn select_candidate(&mut self, index: usize) {
        if let Some(node) = self.current_node()
            && let Some(candidates) = &node.candidates
            && index < candidates.len()
        {
            self.selected_candidate = index;
        }
    }

    pub fn process_key(&mut self, c: char) -> Vec<EngineAction> {
        match (self.active, c) {
            (false, '\\') => {
                self.activate();
                vec![EngineAction::UpdateComposition(self.buffer.clone())]
            }
            (false, _) => vec![EngineAction::Reject],
            // Receiving backslash in active mode
            (true, '\\') => {
                let text = if self.buffer == "\\" {
                    "\\".to_string()
                } else if let candidates = self.get_candidates()
                    && self.selected_candidate < candidates.len()
                {
                    candidates[self.selected_candidate].clone()
                } else {
                    self.buffer.clone()
                };
                self.activate();
                vec![
                    EngineAction::Commit(text),
                    EngineAction::UpdateComposition(self.buffer.clone()),
                ]
            }
            // backspace
            (true, '\x08') | (true, '\x7f') if self.buffer.is_empty() => {
                self.active = false;
                vec![EngineAction::Reject]
            }
            (true, '\x08') | (true, '\x7f') if self.buffer == "\\" => {
                self.deactivate();
                vec![EngineAction::UpdateComposition(String::new())]
            }
            (true, '\x08') | (true, '\x7f') => {
                self.pop();

                if let Some(current) = self.current_node()
                    && let Some(candidates) = &current.candidates
                    && !candidates.is_empty()
                {
                    vec![EngineAction::ShowCandidates(self.buffer.clone())]
                } else {
                    vec![EngineAction::UpdateComposition(self.buffer.clone())]
                }
            }
            (true, c) => {
                let next_node_arc = self
                    .current_node()
                    .and_then(|node| node.children.get(&c.to_string()))
                    .map(|n| Arc::new(n.clone()));

                if let Some(next_node_arc) = next_node_arc {
                    if next_node_arc.children.is_empty() {
                        let candidates = next_node_arc.candidates.as_ref();
                        let text = if candidates.map_or(true, |v| v.is_empty()) {
                            // No children, no candidates: commit buffer + char
                            format!("{}{}", self.buffer, c)
                        } else if let Some(candidates) = candidates
                            && candidates.len() == 1
                        {
                            // No children, one candidate: commit it
                            candidates[0].clone()
                        } else {
                            "".to_string()
                        };
                        if !text.is_empty() {
                            self.deactivate();
                            return vec![EngineAction::Commit(text)];
                        }
                    }

                    self.push(next_node_arc, c);

                    if let Some(current) = self.current_node()
                        && let Some(candidates) = &current.candidates
                        && !candidates.is_empty()
                    {
                        return vec![EngineAction::ShowCandidates(self.buffer.clone())];
                    }

                    vec![EngineAction::UpdateComposition(self.buffer.clone())]
                } else {
                    vec![EngineAction::Reject]
                }
            }
        }
    }

    pub fn activate(&mut self) {
        self.active = true;
        self.buffer = "\\".to_string();
        self.path = vec![Arc::clone(&self.root)];
        self.selected_candidate = 0;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.buffer.clear();
        self.path = vec![Arc::clone(&self.root)];
        self.selected_candidate = 0;
    }

    fn push(&mut self, next_node: Arc<TrieNode>, c: char) {
        self.path.push(next_node);
        self.buffer.push(c);
        self.selected_candidate = 0;
    }

    fn pop(&mut self) {
        self.buffer.pop();
        self.path.pop();
        self.selected_candidate = 0;
    }

    pub fn get_candidates(&self) -> Vec<String> {
        self.current_node()
            .and_then(|node| node.candidates.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test Data Structure
    ///
    /// Patterns covered:
    /// - `\\`: Trigger key logic.
    /// - `\l`: Branch with multiple candidates (["λ", "←"]).
    /// - `\lam`: Leaf node with auto-commit (single candidate "λ").
    /// - `\alpha`: Deep nesting traversal.
    /// - `\b`: Intermediate node with candidate ("β") AND children.
    const TEST_JSON: &str = r#"{ 
        "\\": {
            ">>": ["\\"]
        },
        "=": {
            "=": {
                ">>": ["≡"]
            }
        },
        "<": {
            ">>": ["⟨"]
        },
        ">": {
            ">>": ["⟩"]
        },
        "l": {
            ">>": ["λ", "←"],
            "a": {
                "m": {
                    ">>": ["λ"]
                }
            }
        },
        "a": {
            "l": {
                "p": {
                    "h": {
                        "a": {
                            ">>": ["α"]
                        }
                    }
                }
            }
        },
        "b": {
            ">>": ["β"],
            "e": {
                "t": {
                    "a": {
                        ">>": ["β"]
                    }
                }
            }
        },
        "(": {
            "1": {
                ")": {
                    ">>": ["⑴"]
                }
            }
        }
    }"#;

    #[test]
    fn test_real_world_sequence() {
        let mut engine = Engine::new(TEST_JSON).unwrap();
        // 1. Activate
        engine.process_key('\\'); 

        // 2. Type "==" -> "≡"
        engine.process_key('=');
        let res = engine.process_key('=');
        // This is a leaf node in our test JSON, so it auto-commits "≡"
        assert_eq!(res, vec![EngineAction::Commit("≡".to_string())]);
        assert!(!engine.active); // Leaf commit deactivates

        // 3. Type "\<" -> "⟨"
        engine.process_key('\\');
        let res = engine.process_key('<');
        // Leaf node in test JSON, auto-commits "⟨"
        assert_eq!(res, vec![EngineAction::Commit("⟨".to_string())]);
        assert!(!engine.active);

         // 4. Type "\>" -> "⟩"
        engine.process_key('\\');
        let res = engine.process_key('>');
        assert_eq!(res, vec![EngineAction::Commit("⟩".to_string())]);
        assert!(!engine.active);
    }

    #[test]
    fn test_activation() {
        let mut engine = Engine::new(TEST_JSON).unwrap();
        assert!(!engine.active);

        let res = engine.process_key('\\');
        assert_eq!(res, vec![EngineAction::UpdateComposition("\\".to_string())]);
        assert!(engine.active);
        assert_eq!(engine.buffer, "\\");
    }

    #[test]
    fn test_candidates_display() {
        let mut engine = Engine::new(TEST_JSON).unwrap();
        engine.process_key('\\');

        let res = engine.process_key('l');
        assert_eq!(res, vec![EngineAction::ShowCandidates("\\l".to_string())]);
        assert_eq!(engine.get_candidates(), vec!["λ", "←"]);
    }

    #[test]
    fn test_full_sequence() {
        let mut engine = Engine::new(TEST_JSON).unwrap();
        engine.process_key('\\');
        engine.process_key('l');
        engine.process_key('a');
        let res = engine.process_key('m');

        // \lam -> λ (Leaf auto-commit)
        assert_eq!(res, vec![EngineAction::Commit("λ".to_string())]);
        assert!(!engine.active);
    }

    #[test]
    fn test_implicit_commit_rejection() {
        let mut engine = Engine::new(TEST_JSON).unwrap();
        engine.process_key('\\');
        engine.process_key('l');

        let res = engine.process_key('z');
        assert_eq!(res, vec![EngineAction::Reject]);
        assert!(engine.active);
        assert_eq!(engine.buffer, "\\l");
    }

    #[test]
    fn test_backspace_logic() {
        let mut engine = Engine::new(TEST_JSON).unwrap();
        engine.process_key('\\');
        engine.process_key('l');
        assert_eq!(engine.buffer, "\\l");

        let res = engine.process_key('\x08');
        assert_eq!(res, vec![EngineAction::UpdateComposition("\\".to_string())]);
        assert_eq!(engine.buffer, "\\");
        assert!(engine.active);

        let res = engine.process_key('\x08');
        assert_eq!(res, vec![EngineAction::UpdateComposition("".to_string())]);
        assert!(!engine.active);
    }

    #[test]
    fn test_double_backslash() {
        let mut engine = Engine::new(TEST_JSON).unwrap();
        engine.process_key('\\');
        let res = engine.process_key('\\');

        // Should commit "\" and then UpdateComposition to "\" (restart)
        assert_eq!(
            res,
            vec![
                EngineAction::Commit("\\".to_string()),
                EngineAction::UpdateComposition("\\".to_string())
            ]
        );

        assert!(engine.active);
        assert_eq!(engine.buffer, "\\");
    }

    #[test]
    fn test_deactivate_clears_state() {
        let mut engine = Engine::new(TEST_JSON).unwrap();
        engine.process_key('\\');
        engine.process_key('l');
        assert!(engine.active);
        assert_eq!(engine.buffer, "\\l");

        engine.deactivate();

        assert!(!engine.active);
        assert_eq!(engine.buffer, "");
        let res = engine.process_key('\\');
        assert_eq!(res, vec![EngineAction::UpdateComposition("\\".to_string())]);
    }

    #[test]
    fn test_selection_commit() {
        let mut engine = Engine::new(TEST_JSON).unwrap();
        engine.process_key('\\');
        engine.process_key('l');
        // Candidates: ["λ", "←"]

        // Select second candidate "←" (index 1)
        engine.select_candidate(1);

        // Backslash should commit the selected candidate and restart
        let res = engine.process_key('\\');
        assert_eq!(
            res,
            vec![
                EngineAction::Commit("←".to_string()),
                EngineAction::UpdateComposition("\\".to_string())
            ]
        );

        // Should be reset to active start state
        assert!(engine.active);
        assert_eq!(engine.buffer, "\\");
    }
}
