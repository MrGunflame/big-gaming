//! https://docs.pipewire.org/page_native_protocol.html
//!

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use bytemuck::PodCastError;

const SOCKET_LOCATIONS: &[&str] = &["PIPEWIRE_RUNTIME_DIR", "XDG_RUNTIME_DIR", "USERPROFILE"];
const SOCKET_NAME: &str = "pipewire-0";

pub fn pw_main() {
    let mut stream = UnixStream::connect(socket_addr().unwrap()).unwrap();

    let mut spa_buf = Vec::new();
    PodStruct(PodInt(3)).encode(&mut spa_buf);

    let mut buf = Vec::new();
    // Id
    buf.extend(0_u32.to_ne_bytes());
    // OPCODE
    buf.push(1u8);
    // SIZE
    buf.extend(&[spa_buf.len() as u8, 0, 0]);
    // SEQ
    buf.extend(0_u32.to_ne_bytes());
    // N_FDS
    buf.extend(0_u32.to_ne_bytes());
    // SPA
    buf.extend_from_slice(&spa_buf);

    dbg!(&buf);

    // buf.extend(4_u32.to_ne_bytes());
    // buf.extend(4_u32.to_ne_bytes());
    // buf.extend(3_u32.to_ne_bytes());
    // buf.extend(&[0, 0, 0, 0]);

    stream.write_all(&buf).unwrap();

    let mut resp = vec![0; 100];
    stream.read_to_end(&mut resp).unwrap();

    // let mut buf = Vec::new();
    // // Id
    // buf.extend(1_u32.to_ne_bytes());
    // // OPCODE
    // buf.push(2u8);
    // // SIZE
    // buf.extend(&[8, 0, 0]);
    // // SEQ
    // buf.extend(1_u32.to_ne_bytes());
    // // N_FDS
    // buf.extend(0_u32.to_ne_bytes());
    // // SPA
    // buf.extend(4_u32.to_ne_bytes());
    // buf.extend(4_u32.to_ne_bytes());
    // buf.extend(3_u32.to_ne_bytes());
    // buf.extend(&[0, 0, 0, 0]);

    dbg!(&resp);
}

fn socket_addr() -> Option<PathBuf> {
    for key in SOCKET_LOCATIONS {
        if let Some(path) = std::env::var_os(key) {
            return Some(PathBuf::from(path).join(SOCKET_NAME));
        }
    }

    None
}

trait Pod {
    fn size(&self) -> u32;

    fn kind(&self) -> PodType;

    fn write_payload<B>(&self, buf: B)
    where
        B: BufMut;

    fn encode<B>(&self, buf: B)
    where
        B: BufMut;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
struct PodInt(i32);

impl Pod for PodInt {
    fn size(&self) -> u32 {
        4
    }

    fn kind(&self) -> PodType {
        PodType::Int
    }

    fn write_payload<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.write_i32(self.0);
        // padding
        buf.write_bytes(&[0, 0, 0, 0]);
    }

    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.write_u32(self.size());
        buf.write_u32(PodType::Int as u32);
        buf.write_i32(self.0);
        buf.write_bytes(&[0, 0, 0, 0]);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct PodStruct<T>(T);

impl<T> PodStruct<T>
where
    T: PodIterable,
{
    fn size_of_members(&self) -> u32 {
        (0..self.0.len()).map(|index| self.0.get_size(index)).sum()
    }
}

impl<T> Pod for PodStruct<T>
where
    T: PodIterable,
{
    fn size(&self) -> u32 {
        self.size_of_members() + 8
    }

    fn kind(&self) -> PodType {
        PodType::Struct
    }

    fn write_payload<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        for index in 0..self.0.len() {
            buf.write_u32(self.0.get_size(index));
            buf.write_u32(self.0.get_kind(index) as u32);
            self.0.write_payload_nth(index, &mut buf);
        }
    }

    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.write_u32(self.size_of_members());
        buf.write_u32(PodType::Struct as u32);
        self.write_payload(buf);
    }
}

trait PodIterable {
    fn len(&self) -> usize;
    fn get_kind(&self, index: usize) -> PodType;
    fn get_size(&self, index: usize) -> u32;
    fn write_payload_nth<B>(&self, index: usize, buf: B)
    where
        B: BufMut;
}

impl<T> PodIterable for T
where
    T: Pod,
{
    fn len(&self) -> usize {
        1
    }

    fn get_kind(&self, _index: usize) -> PodType {
        T::kind(self)
    }

    fn get_size(&self, _index: usize) -> u32 {
        T::size(self)
    }

    fn write_payload_nth<B>(&self, _index: usize, buf: B)
    where
        B: BufMut,
    {
        T::write_payload(self, buf);
    }
}

#[derive(Clone, Debug)]
struct PodRepeat<T>(Vec<T>);

impl<T> Pod for PodRepeat<T>
where
    T: Pod,
{
    fn size(&self) -> u32 {
        self.0.iter().map(|v| v.size())
    }
}

enum PodType {
    None = 1,
    Bool = 2,
    Id = 3,
    Int = 4,
    Long = 5,
    Float = 6,
    Double = 7,
    String = 8,
    Bytes = 9,
    Rectangle = 10,
    Fraction = 11,
    Bitmap = 12,
    Array = 13,
    Struct = 14,
    Object = 15,
    Sequence = 16,
    Pointer = 17,
    Fd = 18,
    Choice = 19,
    Pod = 20,
}

trait BufMut {
    fn write_bytes(&mut self, bytes: &[u8]);

    fn write_u8(&mut self, value: u8) {
        self.write_bytes(&[value]);
    }

    fn write_u16(&mut self, value: u16) {
        self.write_bytes(&value.to_ne_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.write_bytes(&value.to_ne_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_ne_bytes());
    }

    fn write_i32(&mut self, value: i32) {
        self.write_bytes(&value.to_ne_bytes());
    }
}

impl<T> BufMut for &mut T
where
    T: BufMut,
{
    fn write_bytes(&mut self, bytes: &[u8]) {
        T::write_bytes(self, bytes);
    }
}

impl BufMut for Vec<u8> {
    fn write_bytes(&mut self, bytes: &[u8]) {
        self.extend_from_slice(bytes);
    }
}
