use super::{BufMut, DecodeState, PodIterable, PodIterableDecoder, PodType};

pub struct Tuple5Decoder<T0, T1, T2, T3, T4>
where
    T0: PodIterable,
    T1: PodIterable,
    T2: PodIterable,
    T3: PodIterable,
    T4: PodIterable,
{
    next: usize,
    t0: T0::Decoder,
    t1: T1::Decoder,
    t2: T2::Decoder,
    t3: T3::Decoder,
    t4: T4::Decoder,
}

impl<T0, T1, T2, T3, T4> Default for Tuple5Decoder<T0, T1, T2, T3, T4>
where
    T0: PodIterable,
    T1: PodIterable,
    T2: PodIterable,
    T3: PodIterable,
    T4: PodIterable,
{
    fn default() -> Self {
        Self {
            next: 0,
            t0: T0::Decoder::default(),
            t1: T1::Decoder::default(),
            t2: T2::Decoder::default(),
            t3: T3::Decoder::default(),
            t4: T4::Decoder::default(),
        }
    }
}

impl<T0, T1, T2, T3, T4> PodIterableDecoder for Tuple5Decoder<T0, T1, T2, T3, T4>
where
    T0: PodIterable,
    T1: PodIterable,
    T2: PodIterable,
    T3: PodIterable,
    T4: PodIterable,
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
                DecodeState::Done => self.next += 1,
            },
            4 => match self.t4.decode(buf) {
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

impl<T0, T1, T2, T3, T4> PodIterable for (T0, T1, T2, T3, T4)
where
    T0: PodIterable,
    T1: PodIterable,
    T2: PodIterable,
    T3: PodIterable,
    T4: PodIterable,
{
    type Decoder = Tuple5Decoder<T0, T1, T2, T3, T4>;

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
        } else if index - self.0.len() - self.1.len() - self.2.len() < self.3.len() {
            self.3
                .get_size(index - self.0.len() - self.1.len() - self.2.len())
        } else {
            self.4
                .get_size(index - self.0.len() - self.1.len() - self.2.len() - self.3.len())
        }
    }

    fn get_kind(&self, index: usize) -> PodType {
        if index < self.0.len() {
            self.0.get_kind(index)
        } else if index - self.0.len() < self.1.len() {
            self.1.get_kind(index - self.0.len())
        } else if index - self.0.len() - self.1.len() < self.2.len() {
            self.2.get_kind(index - self.0.len() - self.1.len())
        } else if index - self.0.len() - self.1.len() < self.3.len() {
            self.3
                .get_kind(index - self.0.len() - self.1.len() - self.2.len())
        } else {
            self.4
                .get_kind(index - self.0.len() - self.1.len() - self.2.len() - self.3.len())
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
        } else if index - self.0.len() - self.1.len() - self.2.len() < self.3.len() {
            self.3
                .write_payload_nth(index - self.0.len() - self.1.len() - self.2.len(), buf);
        } else {
            self.4.write_payload_nth(
                index - self.0.len() - self.1.len() - self.2.len() - self.3.len(),
                buf,
            );
        }
    }

    fn build(decoder: Self::Decoder) -> Self {
        (
            T0::build(decoder.t0),
            T1::build(decoder.t1),
            T2::build(decoder.t2),
            T3::build(decoder.t3),
            T4::build(decoder.t4),
        )
    }
}
