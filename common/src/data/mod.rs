use crate::localization::LocalizedString;

#[derive(Clone, Debug)]
pub struct Item {
    pub id: u64,
    pub name: LocalizedString,
}
