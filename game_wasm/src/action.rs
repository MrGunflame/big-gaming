use crate::encoding::{BinaryReader, Decode};
use crate::host_buffer::{host_buffer, DataBuffer, FieldBuffer};
use crate::record::RecordReference;

pub struct ActionBuffer {
    _priv: (),
}

impl ActionBuffer {
    pub fn get<T>() -> Result<T, T::Error>
    where
        T: Decode,
    {
        let data = host_buffer::<DataBuffer>();
        let fields = host_buffer::<FieldBuffer>();

        let reader = BinaryReader::new(data, fields.into());
        T::decode(reader)
    }
}

pub trait Action: Decode {
    const ID: RecordReference;
}
