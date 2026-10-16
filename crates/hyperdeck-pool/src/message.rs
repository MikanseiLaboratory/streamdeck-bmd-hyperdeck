/// One decoded HyperDeck message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckMessage {
    pub code: u16,
    pub text: String,
    pub body: String,
    pub params: Vec<(String, String)>,
}

impl DeckMessage {
    /// Body parameter, matched by the protocol's lowercase key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    /// `true` for `1xx` failures.
    pub fn is_failure(&self) -> bool {
        (100..200).contains(&self.code)
    }
}
