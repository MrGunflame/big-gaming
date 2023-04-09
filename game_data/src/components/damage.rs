use game_common::module::ModuleId;

#[derive(Copy, Clone, Debug)]
pub struct DamageClass {
    pub module: ModuleId,
    pub id: DamageClassId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DamageClassId(pub u32);
