//! https://docs.pipewire.org/page_native_protocol.html
//!
mod keys;

use std::alloc::dealloc;
use std::collections::HashMap;
use std::ffi::CString;
use std::io::{self, BufRead, Read, Write};
use std::num::Wrapping;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use keys::{PW_KEY_APP_ID, PW_KEY_APP_NAME};

const SOCKET_LOCATIONS: &[&str] = &["PIPEWIRE_RUNTIME_DIR", "XDG_RUNTIME_DIR", "USERPROFILE"];
const SOCKET_NAME: &str = "pipewire-0";

pub fn pw_main() {
    // pipewire::init();
    // let mut mainloop = MainLoop::new(None).unwrap();
    // let context = Context::new(&mainloop).unwrap();
    // let core = context.connect(None).unwrap();

    // mainloop.run();
    // std::thread::park();

    let addr = socket_addr().unwrap();
    let mut client = Client::connect(addr).unwrap();

    client.send(&Hello { version: 3 }).unwrap();

    let info = client.recv().unwrap();
    dbg!(&info);

    client
        .send(&UpdateProperties {
            props: HashMap::from([
                (PW_KEY_APP_ID.to_string(), "hello_world".to_string()),
                (PW_KEY_APP_NAME.to_string(), "hello World".to_string()),
            ]),
        })
        .unwrap();

    let resp = client.recv().unwrap();
    dbg!(&resp);

    client
        .send(&CreateObject {
            factory_name: "node-factory".to_string(),
            new_id: 12345,
            r#type: "Node".to_string(),
            version: 3,
            props: HashMap::new(),
        })
        .unwrap();

    let resp = client.recv().unwrap();
    dbg!(&resp);
    // dbg!(&resp, &header);

    let resp = client.recv().unwrap();
    dbg!(&resp);
    let resp = client.recv().unwrap();
    dbg!(&resp);
    let resp = client.recv().unwrap();
    dbg!(&resp);
    let resp = client.recv().unwrap();
    dbg!(&resp);
    let resp = client.recv().unwrap();
    dbg!(&resp);

    std::thread::park();
}

#[derive(Clone, Debug)]
enum Event {
    Info(Info),
    Done(Done),
    Ping(Ping),
    Error(Error),
    RemoveId(RemoveId),
    BoundId(BoundId),
    ClientInfo(ClientInfo),
}

struct Client {
    stream: UnixStream,
    next_seq: Wrapping<u32>,
}

impl Client {
    fn connect<P>(addr: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let stream = UnixStream::connect(addr)?;
        Ok(Self {
            stream,
            next_seq: Wrapping(0),
        })
    }

    fn recv(&mut self) -> io::Result<Event> {
        let mut buf = [0; 16];
        self.stream.read_exact(&mut buf)?;
        let header = Header::decode(&buf);
        let mut buf = vec![0; header.size as usize];
        self.stream.read_exact(&mut buf)?;

        match header.id {
            0 => match header.opcode {
                0 => Ok(Event::Info(Info::decode(&buf))),
                1 => Ok(Event::Done(Done::decode(&buf))),
                2 => Ok(Event::Ping(Ping::decode(&buf))),
                3 => Ok(Event::Error(Error::decode(&buf))),
                4 => Ok(Event::RemoveId(RemoveId::decode(&buf))),
                5 => Ok(Event::BoundId(BoundId::decode(&buf))),
                n => todo!("{:?}", n),
            },
            1 => match header.opcode {
                0 => Ok(Event::ClientInfo(ClientInfo::decode(&buf))),
                _ => todo!(),
            },
            n => todo!("{:?}", n),
        }
    }

    fn send<T>(&mut self, msg: &T) -> io::Result<()>
    where
        T: Message,
    {
        let mut payload = Vec::new();
        msg.encode(&mut payload);

        let header = Header {
            id: msg.id(),
            opcode: msg.opcode(),
            size: payload.len() as u32,
            seq: self.next_seq.0,
            num_fds: 0,
        };
        self.next_seq += 1;

        let mut buf = Vec::new();
        header.encode(&mut buf);
        buf.extend_from_slice(&payload);
        self.stream.write_all(&buf)?;
        Ok(())
    }
}

fn socket_addr() -> Option<PathBuf> {
    for key in SOCKET_LOCATIONS {
        if let Some(path) = std::env::var_os(key) {
            return Some(PathBuf::from(path).join(SOCKET_NAME));
        }
    }

    None
}

#[derive(Copy, Clone, Debug)]
struct Header {
    id: u32,
    opcode: u8,
    size: u32,
    seq: u32,
    num_fds: u32,
}

impl Header {
    fn encode<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.write_u32(self.id);
        // In the docs it looks like `opnocde` is a single
        // octete that comes before `size` in the stream,
        // but this is actually incorrect.
        // Instead the ordering is dependant on system endianess
        // as `opcode` is stored in the MSBs of the same 4-octets
        // that store the `size`.
        buf.write_u32(u32::from(self.opcode) << 24 | self.size);
        buf.write_u32(self.seq);
        buf.write_u32(self.num_fds);
    }

    fn decode(mut buf: &[u8]) -> Self {
        let id = buf.read_u32();
        let octets = buf.read_u32();
        let opcode = (octets >> 24) as u8;
        let size = octets & 0xFF_FF_FF;
        let seq = buf.read_u32();
        let num_fds = buf.read_u32();
        Self {
            id,
            opcode,
            size,
            seq,
            num_fds,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum CoreEventOpcode {
    Info = 0,
    Done = 1,
    Ping = 2,
    Error = 3,
    RemoveId = 4,
    BoundId = 5,
    AddMem = 6,
    RemoveMem = 7,
    BoundProps = 8,
}

#[derive(Copy, Clone, Debug)]
struct Done {
    id: i32,
    seq: i32,
}

impl Done {
    fn decode(buf: &[u8]) -> Self {
        let value = PodStruct::<(PodInt, PodInt)>::decode(buf).unwrap();
        Self {
            id: value.0 .0 .0,
            seq: value.0 .1 .0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Ping {
    id: i32,
    seq: i32,
}

impl Ping {
    fn decode(buf: &[u8]) -> Self {
        let value = PodStruct::<(PodInt, PodInt)>::decode(buf).unwrap();
        Self {
            id: value.0 .0 .0,
            seq: value.0 .1 .0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct RemoveId {
    id: i32,
}

impl RemoveId {
    fn decode(buf: &[u8]) -> Self {
        let value = PodStruct::<PodInt>::decode(buf).unwrap();
        Self { id: value.0 .0 }
    }
}

#[derive(Copy, Clone, Debug)]
struct BoundId {
    id: i32,
    global_id: i32,
}

impl BoundId {
    fn decode(buf: &[u8]) -> Self {
        let value = PodStruct::<(PodInt, PodInt)>::decode(buf).unwrap();
        Self {
            id: value.0 .0 .0,
            global_id: value.0 .1 .0,
        }
    }
}

#[derive(Clone, Debug)]
struct ClientInfo {
    id: i32,
    change_mask: i64,
    props: HashMap<String, String>,
}

impl ClientInfo {
    fn decode(buf: &[u8]) -> Self {
        let value =
            PodStruct::<(PodInt, PodLong, PodStruct<(PodInt, PodRepeat<PodString>)>)>::decode(buf)
                .unwrap();

        let id = value.0 .0 .0;
        let change_mask = value.0 .1 .0;

        let mut props = HashMap::new();
        let mut props_iter = value.0 .2 .0 .1 .0.into_iter();
        let prop_count = value.0 .2 .0 .0 .0;
        for _ in 0..prop_count {
            let key = props_iter.next().unwrap();
            let val = props_iter.next().unwrap();
            props.insert(
                key.0.to_string_lossy().to_string(),
                val.0.to_string_lossy().to_string(),
            );
        }

        Self {
            id,
            change_mask,
            props,
        }
    }
}

struct GetRegistry {
    version: i32,
    new_id: i32,
}

impl Message for GetRegistry {
    fn id(&self) -> u32 {
        0
    }

    fn opcode(&self) -> u8 {
        5
    }

    fn num_fds(&self) -> u32 {
        0
    }

    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        PodStruct((PodInt(self.version), PodInt(self.new_id))).encode(buf);
    }
}

struct Global {
    id: i32,
    permissions: i32,
    r#type: String,
    version: i32,
    props: HashMap<String, String>,
}

impl Global {
    fn decode(mut buf: &[u8]) -> Self {
        let value = PodStruct::<(
            [PodInt; 2],
            PodString,
            PodInt,
            //PodStruct<(PodInt, PodRepeat<String>)>,
        )>::decode(buf);

        todo!()
    }
}

#[derive(Clone, Debug)]
struct Hello {
    version: i32,
}

impl Message for Hello {
    fn id(&self) -> u32 {
        0
    }

    fn opcode(&self) -> u8 {
        1
    }

    fn num_fds(&self) -> u32 {
        0
    }

    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        PodStruct(PodInt(self.version)).encode(buf);
    }
}

#[derive(Clone, Debug)]
struct Info {
    id: i32,
    cookie: i32,
    user_name: String,
    host_name: String,
    version: String,
    name: String,
    change_mask: i64,
    props: HashMap<String, String>,
}

impl Info {
    fn decode(buf: &[u8]) -> Self {
        let value = PodStruct::<(
            [PodInt; 2],
            [PodString; 4],
            PodLong,
            PodStruct<(PodInt, PodRepeat<PodString>)>,
        )>::decode(buf)
        .unwrap();

        let mut props_iter = value.0 .3 .0 .1 .0.into_iter();
        let mut props = HashMap::new();
        let count = value.0 .3 .0 .0 .0;
        for _ in 0..count {
            let key = props_iter.next().unwrap().0.to_string_lossy().to_string();
            let val = props_iter.next().unwrap().0.to_string_lossy().to_string();
            props.insert(key, val);
        }

        Self {
            id: value.0 .0[0].0,
            cookie: value.0 .0[1].0,
            user_name: value.0 .1[0].0.to_string_lossy().to_string(),
            host_name: value.0 .1[1].0.to_string_lossy().to_string(),
            version: value.0 .1[2].0.to_string_lossy().to_string(),
            name: value.0 .1[3].0.to_string_lossy().to_string(),
            change_mask: value.0 .2 .0,
            props,
        }
    }
}

#[derive(Clone, Debug)]
struct Error {
    id: i32,
    seq: i32,
    res: i32,
    message: String,
}

impl Error {
    fn decode(buf: &[u8]) -> Self {
        let value = PodStruct::<(PodInt, PodInt, PodInt, PodString)>::decode(buf).unwrap();

        Self {
            id: value.0 .0 .0,
            seq: value.0 .1 .0,
            res: value.0 .2 .0,
            message: value.0 .3 .0.to_string_lossy().to_string(),
        }
    }
}

#[derive(Clone, Debug)]
struct UpdateProperties {
    props: HashMap<String, String>,
}

impl Message for UpdateProperties {
    fn id(&self) -> u32 {
        1
    }

    fn opcode(&self) -> u8 {
        2
    }

    fn num_fds(&self) -> u32 {
        0
    }

    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        let mut strings = Vec::new();
        for (key, val) in &self.props {
            strings.push(PodString(CString::new(key.as_bytes()).unwrap()));
            strings.push(PodString(CString::new(val.as_bytes()).unwrap()));
        }

        PodStruct::<PodStruct<(PodInt, PodRepeat<PodString>)>>(PodStruct((
            PodInt(self.props.len() as i32),
            PodRepeat(strings),
        )))
        .encode(buf);
    }
}

#[derive(Clone, Debug)]
struct CreateObject {
    factory_name: String,
    r#type: String,
    version: i32,
    props: HashMap<String, String>,
    new_id: i32,
}

impl Message for CreateObject {
    fn id(&self) -> u32 {
        0
    }

    fn opcode(&self) -> u8 {
        6
    }

    fn num_fds(&self) -> u32 {
        0
    }

    fn encode<B>(&self, buf: B)
    where
        B: BufMut,
    {
        let mut strings = Vec::new();
        for (key, val) in &self.props {
            strings.push(PodString(CString::new(key.as_bytes()).unwrap()));
            strings.push(PodString(CString::new(val.as_bytes()).unwrap()));
        }

        PodStruct((
            [
                PodString(CString::new(self.factory_name.as_bytes()).unwrap()),
                PodString(CString::new(self.r#type.as_bytes()).unwrap()),
            ],
            PodInt(self.version),
            PodStruct((PodInt(self.props.len() as i32), PodRepeat(strings))),
            PodInt(self.new_id),
        ))
        .encode(buf);
    }
}

trait Message {
    fn id(&self) -> u32;
    fn opcode(&self) -> u8;
    fn num_fds(&self) -> u32;

    fn encode<B>(&self, buf: B)
    where
        B: BufMut;
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
struct PodLong(i64);

impl Pod for PodLong {
    fn size(&self) -> u32 {
        8
    }

    fn kind(&self) -> PodType {
        PodType::Long
    }

    fn write_payload<B>(&self, mut buf: B)
    where
        B: BufMut,
    {
        buf.write_i64(self.0);
    }

    fn read_payload(mut buf: &[u8]) -> Self {
        let value = buf.read_i64();
        Self(value)
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
        self.size_of_members()
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

        while !buf.is_empty() {
            let size = align_up(buf.read_u32());
            let kind = buf.read_u32();

            let (elem, rem) = buf.split_at(size as usize);
            buf = rem;

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

struct Tuple3Decoder<T0, T1, T2>
where
    T0: PodIterable,
    T1: PodIterable,
    T2: PodIterable,
{
    next: usize,
    t0: T0::Decoder,
    t1: T1::Decoder,
    t2: T2::Decoder,
}

impl<T0, T1, T2> Default for Tuple3Decoder<T0, T1, T2>
where
    T0: PodIterable,
    T1: PodIterable,
    T2: PodIterable,
{
    fn default() -> Self {
        Self {
            next: 0,
            t0: T0::Decoder::default(),
            t1: T1::Decoder::default(),
            t2: T2::Decoder::default(),
        }
    }
}

impl<T0, T1, T2> PodIterableDecoder for Tuple3Decoder<T0, T1, T2>
where
    T0: PodIterable,
    T1: PodIterable,
    T2: PodIterable,
{
    fn decode(&mut self, buf: &[u8]) -> DecodeState {
        match self.next {
            0 => match self.t0.decode(buf) {
                DecodeState::NeedMore => (),
                DecodeState::Done => self.next += 1,
            },
            1 => match self.t1.decode(buf) {
                DecodeState::NeedMore => (),
                DecodeState::Done => self.next += 1,
            },
            2 => match self.t2.decode(buf) {
                DecodeState::NeedMore => (),
                DecodeState::Done => {
                    self.next += 1;
                    return DecodeState::Done;
                }
            },
            _ => unreachable!(),
        }

        DecodeState::NeedMore
    }
}

impl<T0, T1, T2> PodIterable for (T0, T1, T2)
where
    T0: PodIterable,
    T1: PodIterable,
    T2: PodIterable,
{
    type Decoder = Tuple3Decoder<T0, T1, T2>;

    fn len(&self) -> usize {
        self.0.len() + self.1.len() + self.2.len()
    }

    fn get_size(&self, index: usize) -> u32 {
        if index < self.0.len() {
            self.0.get_size(index)
        } else if index - self.0.len() < self.1.len() {
            self.1.get_size(index - self.0.len())
        } else {
            self.2.get_size(index - self.0.len() - self.1.len())
        }
    }

    fn get_kind(&self, index: usize) -> PodType {
        if index < self.0.len() {
            self.0.get_kind(index)
        } else if index - self.0.len() < self.1.len() {
            self.1.get_kind(index - self.0.len())
        } else {
            self.2.get_kind(index - self.0.len() - self.1.len())
        }
    }

    fn write_payload_nth<B>(&self, index: usize, buf: B)
    where
        B: BufMut,
    {
        if index < self.0.len() {
            self.0.write_payload_nth(index, buf);
        } else if index - self.0.len() < self.1.len() {
            self.1.write_payload_nth(index - self.0.len(), buf);
        } else {
            self.2
                .write_payload_nth(index - self.0.len() - self.1.len(), buf);
        }
    }

    fn build(decoder: Self::Decoder) -> Self {
        (
            T0::build(decoder.t0),
            T1::build(decoder.t1),
            T2::build(decoder.t2),
        )
    }
}

struct Tuple4Decoder<T0, T1, T2, T3>
where
    T0: PodIterable,
    T1: PodIterable,
    T2: PodIterable,
    T3: PodIterable,
{
    next: usize,
    t0: T0::Decoder,
    t1: T1::Decoder,
    t2: T2::Decoder,
    t3: T3::Decoder,
}

impl<T0, T1, T2, T3> Default for Tuple4Decoder<T0, T1, T2, T3>
where
    T0: PodIterable,
    T1: PodIterable,
    T2: PodIterable,
    T3: PodIterable,
{
    fn default() -> Self {
        Self {
            next: 0,
            t0: T0::Decoder::default(),
            t1: T1::Decoder::default(),
            t2: T2::Decoder::default(),
            t3: T3::Decoder::default(),
        }
    }
}

impl<T0, T1, T2, T3> PodIterableDecoder for Tuple4Decoder<T0, T1, T2, T3>
where
    T0: PodIterable,
    T1: PodIterable,
    T2: PodIterable,
    T3: PodIterable,
{
    fn decode(&mut self, buf: &[u8]) -> DecodeState {
        match self.next {
            0 => match self.t0.decode(buf) {
                DecodeState::NeedMore => (),
                DecodeState::Done => self.next += 1,
            },
            1 => match self.t1.decode(buf) {
                DecodeState::NeedMore => (),
                DecodeState::Done => self.next += 1,
            },
            2 => match self.t2.decode(buf) {
                DecodeState::NeedMore => (),
                DecodeState::Done => self.next += 1,
            },
            3 => match self.t3.decode(buf) {
                DecodeState::NeedMore => (),
                DecodeState::Done => {
                    self.next += 1;
                    return DecodeState::Done;
                }
            },
            _ => unreachable!(),
        }

        DecodeState::NeedMore
    }
}

impl<T0, T1, T2, T3> PodIterable for (T0, T1, T2, T3)
where
    T0: PodIterable,
    T1: PodIterable,
    T2: PodIterable,
    T3: PodIterable,
{
    type Decoder = Tuple4Decoder<T0, T1, T2, T3>;

    fn len(&self) -> usize {
        self.0.len() + self.1.len() + self.2.len() + self.3.len()
    }

    fn get_size(&self, index: usize) -> u32 {
        if index < self.0.len() {
            self.0.get_size(index)
        } else if index - self.0.len() < self.1.len() {
            self.1.get_size(index - self.0.len())
        } else if index - self.0.len() - self.1.len() < self.2.len() {
            self.2.get_size(index - self.0.len() - self.1.len())
        } else {
            self.3
                .get_size(index - self.0.len() - self.1.len() - self.2.len())
        }
    }

    fn get_kind(&self, index: usize) -> PodType {
        if index < self.0.len() {
            self.0.get_kind(index)
        } else if index - self.0.len() < self.1.len() {
            self.1.get_kind(index - self.0.len())
        } else if index - self.0.len() - self.1.len() < self.2.len() {
            self.2.get_kind(index - self.0.len() - self.1.len())
        } else {
            self.3
                .get_kind(index - self.0.len() - self.1.len() - self.2.len())
        }
    }

    fn write_payload_nth<B>(&self, index: usize, buf: B)
    where
        B: BufMut,
    {
        if index < self.0.len() {
            self.0.write_payload_nth(index, buf);
        } else if index - self.0.len() < self.1.len() {
            self.1.write_payload_nth(index - self.0.len(), buf);
        } else if index - self.0.len() - self.1.len() < self.2.len() {
            self.2
                .write_payload_nth(index - self.0.len() - self.1.len(), buf);
        } else {
            self.3
                .write_payload_nth(index - self.0.len() - self.1.len() - self.2.len(), buf);
        }
    }

    fn build(decoder: Self::Decoder) -> Self {
        (
            T0::build(decoder.t0),
            T1::build(decoder.t1),
            T2::build(decoder.t2),
            T3::build(decoder.t3),
        )
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
        for _ in 0..padding_for(bytes.len() as u32) {
            buf.write_u8(0);
        }
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

    fn write_i64(&mut self, value: i64) {
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

    fn read_i64(&mut self) -> i64 {
        let mut buf = [0; 8];
        self.read_slice(&mut buf);
        i64::from_ne_bytes(buf)
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

fn padding_for(size: u32) -> u32 {
    if size % 8 == 0 {
        0
    } else {
        8 - (size % 8)
    }
}
