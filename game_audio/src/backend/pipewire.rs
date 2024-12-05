//! https://docs.pipewire.org/page_native_protocol.html
//!

use std::ffi::CString;
use std::io::{BufRead, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use bytemuck::PodCastError;
use game_common::components::Encode;
use libspa::pod::builder::Builder;
use pipewire::context::Context;
use pipewire::main_loop::MainLoop;
use rustix::net::{SendAncillaryBuffer, SendFlags};

use crate::manager;

const SOCKET_LOCATIONS: &[&str] = &["PIPEWIRE_RUNTIME_DIR", "XDG_RUNTIME_DIR", "USERPROFILE"];
const SOCKET_NAME: &str = "pipewire-0";

pub fn pw_main() {
    // pipewire::init();
    // let mut mainloop = MainLoop::new(None).unwrap();
    // let context = Context::new(&mainloop).unwrap();
    // let core = context.connect(None).unwrap();

    // mainloop.run();
    // std::thread::park();

    let mut stream = UnixStream::connect(socket_addr().unwrap()).unwrap();

    let mut spa_buf = Vec::new();
    PodStruct(PodInt(3)).encode(&mut spa_buf);

    let mut buf = Vec::new();
    // Id
    buf.extend(0_u32.to_ne_bytes());
    // SIZE
    buf.extend(&[spa_buf.len() as u8, 0, 0]);
    // OPCODE
    buf.push(1u8);
    // SEQ
    buf.extend(0_u32.to_ne_bytes());
    // N_FDS
    buf.extend(0_u32.to_ne_bytes());
    // SPA
    buf.extend_from_slice(&spa_buf);

    // dbg!(&spa_buf.len());

    // dbg!(&buf);

    // buf.extend(4_u32.to_ne_bytes());
    // buf.extend(4_u32.to_ne_bytes());
    // buf.extend(3_u32.to_ne_bytes());
    // buf.extend(&[0, 0, 0, 0]);

    stream.write_all(&buf).unwrap();

    let mut resp = vec![0; 200];
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    stream.read(&mut resp).unwrap();
    // dbg!(&resp);
    // return;

    let info = PodStruct::<([PodInt; 2], [PodString; 4])>::decode(&resp[16..]).unwrap();
    dbg!(&info);

    return;

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
    let mut spa_buf2 = Vec::new();
    let client_props = PodStruct(PodStruct((
        PodInt(1),
        PodRepeat(vec![
            PodString(CString::from(c"application.id")),
            PodString(CString::from(c"hello_world_69")),
        ]),
    )));
    client_props.encode(&mut spa_buf2);
    // dbg!(spa_buf2.len());

    let mut buf = Vec::new();
    // Id
    buf.extend(1_u32.to_ne_bytes());
    // OPCODE
    buf.push(2u8);
    // SIZE
    buf.extend(&[spa_buf2.len() as u8, 0, 0]);
    // SEQ
    buf.extend(1_u32.to_ne_bytes());
    // N_FDS
    buf.extend(0_u32.to_ne_bytes());
    // SPA
    buf.extend_from_slice(&spa_buf2);

    stream.write_all(&buf).unwrap();

    // let mut resp = vec![0; 100];
    // stream.read_to_end(&mut resp).unwrap();

    // dbg!(&resp);
}

fn socket_addr() -> Option<PathBuf> {
    for key in SOCKET_LOCATIONS {
        if let Some(path) = std::env::var_os(key) {
            return Some(PathBuf::from(path).join(SOCKET_NAME));
        }
    }

    None
}

struct Hello {
    version: i32,
}

impl Hello {
    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        PodStruct(PodInt(self.version)).encode(buf);
    }
}

trait Pod: Sized {
    fn size(&self) -> u32;

    fn kind(&self) -> PodType;

    fn write_payload<B>(&self, buf: B)
    where
        B: BufMut;

    fn read_payload(buf: &[u8]) -> Self;

    fn encode<B>(&self, buf: B)
    where
        B: BufMut;

    fn decode(buf: &[u8]) -> Result<Self, ()>;
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

    fn read_payload(mut buf: &[u8]) -> Self {
        let value = buf.read_i32();
        buf.read_u32();
        Self(value)
    }

    fn decode(buf: &[u8]) -> Result<Self, ()> {
        todo!()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct PodStruct<T>(T);

impl<T> PodStruct<T>
where
    T: PodIterable,
{
    fn size_of_members(&self) -> u32 {
        (0..self.0.len())
            .map(|index| align_up(self.0.get_size(index)) + 4 + 4)
            .sum()
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

    fn read_payload(mut buf: &[u8]) -> Self {
        let mut decoder = T::Decoder::default();

        dbg!(&buf);

        while !buf.is_empty() {
            let size = align_up(buf.read_u32());
            let kind = buf.read_u32();
            dbg!(&size, &kind);

            if kind > 20 {
                break;
            }

            let (elem, rem) = buf.split_at(size as usize);
            buf = rem;

            dbg!(&elem);

            let res = decoder.decode(elem);
            if matches!(res, DecodeState::Done) {
                break;
            }
        }

        Self(T::build(decoder))
    }

    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.write_u32(self.size_of_members());
        buf.write_u32(PodType::Struct as u32);
        self.write_payload(buf);
    }

    fn decode(mut buf: &[u8]) -> Result<Self, ()> {
        let size = buf.read_u32();
        let kind = buf.read_u32();
        dbg!(&kind, &size);
        Ok(Self::read_payload(buf))
    }
}

trait PodIterable {
    type Decoder: Default + PodIterableDecoder;

    fn len(&self) -> usize;
    fn get_kind(&self, index: usize) -> PodType;
    fn get_size(&self, index: usize) -> u32;
    fn write_payload_nth<B>(&self, index: usize, buf: B)
    where
        B: BufMut;

    fn build(decoder: Self::Decoder) -> Self;
}

struct PodDecoder<T>(Option<T>);

impl<T> PodIterableDecoder for PodDecoder<T>
where
    T: Pod,
{
    fn decode(&mut self, buf: &[u8]) -> DecodeState {
        self.0 = Some(T::read_payload(buf));
        DecodeState::Done
    }
}

impl<T> Default for PodDecoder<T> {
    fn default() -> Self {
        Self(None)
    }
}

trait PodIterableDecoder {
    fn decode(&mut self, buf: &[u8]) -> DecodeState;
}

impl<T> PodIterable for T
where
    T: Pod,
{
    type Decoder = PodDecoder<Self>;

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

    fn build(decoder: Self::Decoder) -> Self {
        decoder.0.unwrap()
    }
}

struct Tuple2Decoder<T, U>
where
    T: PodIterable,
    U: PodIterable,
{
    next: usize,
    decoder0: T::Decoder,
    decoder1: U::Decoder,
}

impl<T, U> Default for Tuple2Decoder<T, U>
where
    T: PodIterable,
    U: PodIterable,
{
    fn default() -> Self {
        Self {
            next: 0,
            decoder0: T::Decoder::default(),
            decoder1: U::Decoder::default(),
        }
    }
}

impl<T, U> PodIterableDecoder for Tuple2Decoder<T, U>
where
    T: PodIterable,
    U: PodIterable,
{
    fn decode(&mut self, buf: &[u8]) -> DecodeState {
        match self.next {
            0 => match self.decoder0.decode(buf) {
                DecodeState::NeedMore => (),
                DecodeState::Done => self.next += 1,
            },
            1 => match self.decoder1.decode(buf) {
                DecodeState::NeedMore => (),
                DecodeState::Done => {
                    self.next += 1;
                    return DecodeState::Done;
                }
            },
            _ => return DecodeState::Done,
        }

        DecodeState::NeedMore
    }
}

#[derive(Copy, Clone, Debug)]
enum DecodeState {
    NeedMore,
    Done,
}

impl<T, U> PodIterable for (T, U)
where
    T: PodIterable,
    U: PodIterable,
{
    type Decoder = Tuple2Decoder<T, U>;

    fn len(&self) -> usize {
        self.0.len() + self.1.len()
    }

    fn get_size(&self, index: usize) -> u32 {
        if index < self.0.len() {
            self.0.get_size(index)
        } else {
            self.1.get_size(index - self.0.len())
        }
    }

    fn get_kind(&self, index: usize) -> PodType {
        if index < self.0.len() {
            self.0.get_kind(index)
        } else {
            self.1.get_kind(index - self.0.len())
        }
    }

    fn write_payload_nth<B>(&self, index: usize, buf: B)
    where
        B: BufMut,
    {
        if index < self.0.len() {
            self.0.write_payload_nth(index, buf);
        } else {
            self.1.write_payload_nth(index - self.0.len(), buf);
        }
    }

    fn build(decoder: Self::Decoder) -> Self {
        (T::build(decoder.decoder0), U::build(decoder.decoder1))
    }
}

struct ArrayDecoder<T, const N: usize>(Vec<T>);

impl<T, const N: usize> Default for ArrayDecoder<T, N> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T, const N: usize> PodIterableDecoder for ArrayDecoder<T, N>
where
    T: Pod,
{
    fn decode(&mut self, buf: &[u8]) -> DecodeState {
        self.0.push(T::read_payload(buf));
        if self.0.len() != N {
            DecodeState::NeedMore
        } else {
            DecodeState::Done
        }
    }
}

impl<T, const N: usize> PodIterable for [T; N]
where
    T: Pod + std::fmt::Debug,
{
    type Decoder = ArrayDecoder<T, N>;

    fn len(&self) -> usize {
        N
    }

    fn get_size(&self, index: usize) -> u32 {
        self[index].size()
    }

    fn get_kind(&self, index: usize) -> PodType {
        self[index].kind()
    }

    fn write_payload_nth<B>(&self, index: usize, buf: B)
    where
        B: BufMut,
    {
        self[index].write_payload(buf);
    }

    fn build(decoder: Self::Decoder) -> Self {
        decoder.0.try_into().unwrap()
    }
}

struct RepeatDecoder<T>(Vec<T>);

impl<T> Default for RepeatDecoder<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> PodIterableDecoder for RepeatDecoder<T>
where
    T: Pod,
{
    fn decode(&mut self, buf: &[u8]) -> DecodeState {
        self.0.push(T::read_payload(buf));
        DecodeState::NeedMore
    }
}

#[derive(Clone, Debug)]
struct PodRepeat<T>(Vec<T>);

impl<T> PodIterable for PodRepeat<T>
where
    T: Pod,
{
    type Decoder = RepeatDecoder<T>;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn get_size(&self, index: usize) -> u32 {
        self.0[index].size()
    }

    fn get_kind(&self, index: usize) -> PodType {
        self.0[index].kind()
    }

    fn write_payload_nth<B>(&self, index: usize, buf: B)
    where
        B: BufMut,
    {
        self.0[index].write_payload(buf);
    }

    fn build(decoder: Self::Decoder) -> Self {
        Self(decoder.0)
    }
}

#[derive(Clone, Debug)]
struct PodString(CString);

impl Pod for PodString {
    fn size(&self) -> u32 {
        self.0.as_bytes_with_nul().len() as u32
    }

    fn kind(&self) -> PodType {
        PodType::String
    }

    fn write_payload<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        let bytes = self.0.as_bytes_with_nul();
        buf.write_bytes(bytes);
    }

    fn read_payload(mut buf: &[u8]) -> Self {
        let mut bytes = Vec::new();
        buf.read_until(0, &mut bytes).unwrap();
        Self(CString::from_vec_with_nul(bytes).unwrap())
    }

    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        todo!()
    }

    fn decode(buf: &[u8]) -> Result<Self, ()> {
        todo!()
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

trait Buf {
    fn read_slice(&mut self, buf: &mut [u8]);

    fn read_u8(&mut self) -> u8 {
        let mut buf = [0];
        self.read_slice(&mut buf);
        buf[0]
    }

    fn read_u16(&mut self) -> u16 {
        let mut buf = [0; 2];
        self.read_slice(&mut buf);
        u16::from_ne_bytes(buf)
    }

    fn read_u32(&mut self) -> u32 {
        let mut buf = [0; 4];
        self.read_slice(&mut buf);
        u32::from_ne_bytes(buf)
    }

    fn read_i32(&mut self) -> i32 {
        let mut buf = [0; 4];
        self.read_slice(&mut buf);
        i32::from_ne_bytes(buf)
    }
}

impl Buf for &[u8] {
    fn read_slice(&mut self, buf: &mut [u8]) {
        buf.copy_from_slice(&self[..buf.len()]);
        *self = &self[buf.len()..];
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

fn align_up(size: u32) -> u32 {
    if size % 8 == 0 {
        size
    } else {
        size + (8 - (size % 8))
    }
}
