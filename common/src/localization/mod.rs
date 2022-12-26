use std::fmt::{self, Display, Formatter};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LocalizedString(String);

impl LocalizedString {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for LocalizedString {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl AsRef<str> for LocalizedString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
