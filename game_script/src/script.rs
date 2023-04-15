use game_data::uri::Uri;

#[derive(Debug)]
pub struct Script {
    pub kind: ScriptKind,
    pub path: Uri,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ScriptKind {
    Native,
}
