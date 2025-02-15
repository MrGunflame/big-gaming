pub mod channel;
pub mod socket;

mod loss_list;
mod reassembly;

use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::marker::PhantomData;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, LazyLock};
use std::task::{ready, Context, Poll};
use std::time::{Duration, Instant};

use futures::future::FusedFuture;
use futures::{Sink, SinkExt, Stream, StreamExt};
use game_common::world::control_frame::ControlFrame;
use game_tracing::trace_span;
use loss_list::LossList;
use parking_lot::Mutex;
use rand::rngs::OsRng;
use rand::Rng;
use reassembly::ReassemblyBuffer;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::time::MissedTickBehavior;

use crate::message::{ControlMessage, DataMessage, DataMessageBody, Message, MessageId};
use crate::proto::ack::{Ack, AckAck, Nak};
use crate::proto::handshake::{Handshake, HandshakeFlags, HandshakeType};
use crate::proto::sequence::Sequence;
use crate::proto::shutdown::{Shutdown, ShutdownReason};
use crate::proto::{
    Decode, Encode, Flags, Frame, Header, Packet, PacketBody, PacketPosition, PacketType,
    SequenceRange,
};

/// Max bytes that be stored for reassembly.
const MAX_REASSEMBLY_SIZE: usize = 16777216;

#[derive(Clone, Debug, Error)]
pub enum Error<E>
where
    E: std::error::Error,
{
    #[error("peer timed out")]
    Timeout,
    #[error("stream error: {0}")]
    Stream(#[from] E),
}

/// A bidirectional stream underlying a [`Connection`].
// FIXME: We don't need `Unpin`.
pub trait ConnectionStream: Stream<Item = Packet> + Sink<Packet> + Unpin {
    const IS_RELIABLE: bool;
    const IS_ORDERED: bool;
}

pub struct Connection<S, M>
where
    S: ConnectionStream,
    S::Error: std::error::Error,
    M: ConnectionMode,
{
    stream: S,

    reader: mpsc::Receiver<Message>,
    writer: mpsc::Sender<Message>,

    state: ConnectionState,
    interval: TickInterval,
    last_time: Instant,

    packet_queue: VecDeque<Packet>,
    frame_queue: VecDeque<(Frame, ControlFrame, MessageId)>,

    next_local_sequence: Sequence,
    next_peer_sequence: Sequence,
    next_ack_sequence: Sequence,

    /// List of lost packets from the peer.
    loss_list: LossList,

    /// Packets that have been sent and are buffered until an ACK is received for them.
    inflight_packets: InflightPackets,

    /// Starting control frame.
    start_control_frame: ControlFrame,

    /// Local constant buffer in control frames.
    const_delay: u16,

    _mode: PhantomData<fn() -> M>,

    #[cfg(debug_assertions)]
    debug_validator: crate::validator::DebugValidator,

    max_data_size: u16,

    reassembly_buffer: ReassemblyBuffer,
    ack_time_list: AckTimeList,
    rtt: Arc<Mutex<Rtt>>,
    /// Last Processed control frame
    last_cf: ControlFrame,
    message_out: HashMap<Sequence, MessageId>,
    // MessageId => last sequence
    messages_in: HashMap<MessageId, Sequence>,
    next_id: u32,
    is_writing: bool,

    local_addr: SocketAddr,
    remote_addr: SocketAddr,
}

impl<S, M> Connection<S, M>
where
    S: ConnectionStream,
    S::Error: std::error::Error,
    M: ConnectionMode,
{
    pub fn new(
        stream: S,
        control_frame: ControlFrame,
        const_delay: ControlFrame,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
    ) -> (Self, ConnectionHandle) {
        let (out_tx, out_rx) = mpsc::channel(4096);
        let (writer, reader) = mpsc::channel(4096);

        let rtt = Arc::new(Mutex::new(Rtt::new()));

        let mut conn = Self {
            stream,
            state: ConnectionState::Handshake(HandshakeState::Hello),
            reader: out_rx,
            writer,
            packet_queue: VecDeque::new(),
            frame_queue: VecDeque::new(),
            interval: TickInterval::new(),
            last_time: Instant::now(),
            next_local_sequence: Sequence::default(),
            next_ack_sequence: Sequence::default(),
            next_peer_sequence: Sequence::default(),
            loss_list: LossList::new(8192),
            inflight_packets: InflightPackets::new(8192),

            start_control_frame: control_frame,

            _mode: PhantomData,

            const_delay: const_delay.0,

            max_data_size: 512,
            reassembly_buffer: ReassemblyBuffer::new(MAX_REASSEMBLY_SIZE),

            #[cfg(debug_assertions)]
            debug_validator: crate::validator::DebugValidator::new(),

            ack_time_list: AckTimeList::new(),
            rtt: rtt.clone(),
            last_cf: ControlFrame(0),
            message_out: HashMap::new(),
            messages_in: HashMap::new(),
            next_id: 0,
            is_writing: false,
            local_addr,
            remote_addr,
        };

        if M::IS_CONNECT {
            conn.prepare_connect();
        }

        (
            conn,
            ConnectionHandle {
                chan_out: out_tx,
                rx: Mutex::new(reader),
                rtt,
            },
        )
    }

    fn poll_read(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error<S::Error>>> {
        let _span = trace_span!("Connection::poll_read").entered();

        // Flush the send buffer before reading any packets.
        if !self.packet_queue.is_empty() {
            self.init_write()?;
            return Poll::Ready(Ok(()));
        }

        while let Poll::Ready(packet) = self.stream.poll_next_unpin(cx) {
            self.last_time = Instant::now();

            let Some(packet) = packet else {
                // `None` means the remote peer has hung up and will
                // no longer send/receive packets.
                self.abort();
                return Poll::Ready(Ok(()));
            };

            if self.handle_packet(packet).is_ready() {
                return Poll::Ready(Ok(()));
            }
        }

        if let Poll::Ready(res) = self.poll_tick(cx) {
            return Poll::Ready(res);
        }

        // Don't send commands to peer until connected.
        if self.state != ConnectionState::Connected {
            return Poll::Pending;
        }

        let mut packets_written = false;
        let mut frames_written = false;
        while let Poll::Ready(msg) = self.reader.poll_recv(cx) {
            let Some(msg) = msg else {
                self.shutdown();
                return Poll::Ready(Ok(()));
            };

            match msg {
                Message::Control(ControlMessage::Acknowledge(id, cf)) => {
                    if let Some(seq) = self.messages_in.remove(&id) {
                        let packet = Packet {
                            header: Header {
                                packet_type: PacketType::ACK,
                                sequence: Sequence::new(0),
                                control_frame: cf - self.start_control_frame,
                                flags: Flags::new(),
                            },
                            body: PacketBody::Ack(Ack {
                                sequence: seq,
                                ack_sequence: self.next_ack_sequence.fetch_next(),
                            }),
                        };

                        self.packet_queue.push_back(packet);
                        packets_written = true;
                    }
                }
                Message::Control(ControlMessage::Ack(cf)) => {
                    self.last_cf = cf - self.start_control_frame;
                }
                Message::Data(msg) => {
                    let id = msg.id;
                    let cf = msg.control_frame - self.start_control_frame;
                    let frame = msg.body.into_frame();
                    self.frame_queue.push_back((frame, cf, id));
                    frames_written = true;
                }
                _ => unreachable!(),
            }
        }

        if frames_written {
            self.write_snapshot();
            packets_written = true;
        }

        if packets_written {
            return Poll::Ready(Ok(()));
        } else {
            Poll::Pending
        }
    }

    fn write_snapshot(&mut self) {
        // Merge FrameQueue into FrameBuffer, compact then send.

        while let Some((frame, cf, id)) = self.frame_queue.pop_front() {
            // Track the last sequence. `fragment_frame` always calls the closure
            // at least once; for solo commands we only need to track the single
            // sequence, but for fragmented commands we track the last sequence
            // of the stream.
            let mut last_seq = Sequence::default();

            fragment_frame(
                &frame,
                &mut self.next_local_sequence,
                cf,
                self.max_data_size,
                |packet| {
                    last_seq = packet.header.sequence;

                    self.inflight_packets.insert(packet.clone());
                    self.packet_queue.push_back(packet);
                },
            );

            // Frames are only received once all packets for that frame arrived.
            // Therefore we only care about the last sequence in the chain.
            self.message_out.insert(last_seq, id);
        }
    }

    fn init_write(&mut self) -> Result<(), S::Error> {
        self.is_writing = true;
        let packet = self.packet_queue.pop_front().unwrap();
        self.stream.start_send_unpin(packet)
    }

    fn poll_write(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), S::Error>> {
        let _span = trace_span!("Connection::poll_write").entered();

        self.stream.poll_ready_unpin(cx)
    }

    ///
    /// Returns `Poll::Ready` on state change.
    fn handle_packet(&mut self, packet: Packet) -> Poll<Result<(), Error<S::Error>>> {
        match packet.body {
            PacketBody::Handshake(body) => self.handle_handshake(packet.header, body),
            PacketBody::Shutdown(body) => self.handle_shutdown(packet.header, body),
            PacketBody::Ack(body) => self.handle_ack(packet.header, body),
            PacketBody::AckAck(body) => self.handle_ackack(packet.header, body),
            PacketBody::Nak(body) => self.handle_nak(packet.header, body),
            PacketBody::Data(body) => self.handle_data(packet.header, body),
        }
    }

    fn handle_data(&mut self, header: Header, body: Vec<u8>) -> Poll<Result<(), Error<S::Error>>> {
        // Drop out-of-order or duplicates.
        if header.sequence < self.next_peer_sequence && !self.loss_list.remove(header.sequence) {
            tracing::warn!("dropping duplicate packet {:?}", header.sequence);
            return Poll::Ready(Ok(()));
        }

        // Prepare NAK if we lost a packet.
        let nak = if self.next_peer_sequence != header.sequence {
            Some(Packet {
                header: Header {
                    packet_type: PacketType::NAK,
                    sequence: Sequence::new(0),
                    control_frame: ControlFrame(0),
                    flags: Flags::new(),
                },
                body: PacketBody::Nak(Nak {
                    sequences: SequenceRange {
                        start: self.next_peer_sequence,
                        end: header.sequence - 1,
                    },
                }),
            })
        } else {
            None
        };

        // We lost all segments from self.next_peer_sequence..header.sequence.
        while self.next_peer_sequence != header.sequence {
            self.loss_list.insert(self.next_peer_sequence);
            self.next_peer_sequence += 1;
        }
        self.next_peer_sequence += 1;

        // Caught up to most recent packet.
        debug_assert_eq!(self.next_peer_sequence, header.sequence + 1);

        if let Some((seq, payload)) =
            self.reassembly_buffer
                .insert(header.sequence, header.flags.packet_position(), body)
        {
            match Frame::decode(&payload[..]) {
                Ok(frame) => {
                    #[cfg(debug_assertions)]
                    self.debug_validator.push(header, &frame);

                    // Convert back to local control frame.
                    let control_frame = header.control_frame + self.start_control_frame;

                    let id = MessageId(self.next_id);
                    self.next_id = self.next_id.wrapping_add(1);
                    self.messages_in.insert(id, seq);

                    let body = DataMessageBody::from_frame(frame);
                    self.writer
                        .try_send(Message::Data(DataMessage {
                            id,
                            control_frame,
                            body,
                        }))
                        .unwrap();
                }
                Err(err) => {
                    tracing::debug!("failed to decode frame: {}", err);
                }
            }
        }

        if let Some(nak) = nak {
            self.packet_queue.push_back(nak);
            self.state = ConnectionState::Connected;
            Poll::Ready(Ok(()))
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn handle_handshake(
        &mut self,
        _header: Header,
        body: Handshake,
    ) -> Poll<Result<(), Error<S::Error>>> {
        // Ignore if not in HS process.
        let ConnectionState::Handshake(state) = self.state else {
            return Poll::Ready(Ok(()));
        };

        match state {
            // Connect mode
            HandshakeState::Hello if M::IS_CONNECT => {
                if body.kind != HandshakeType::HELLO {
                    tracing::info!("abort: expected HELLO, but got {:?}", body.kind);
                    self.abort();
                    return Poll::Ready(Ok(()));
                }

                // Send AGREEMENT
                let resp = Packet {
                    header: Header {
                        packet_type: PacketType::HANDSHAKE,
                        sequence: Sequence::default(),
                        control_frame: ControlFrame(0),
                        flags: Flags::new(),
                    },
                    body: PacketBody::Handshake(Handshake {
                        version: 0,
                        kind: HandshakeType::AGREEMENT,
                        flags: HandshakeFlags::default(),
                        mtu: 1500,
                        flow_window: 8192,
                        initial_sequence: self.next_local_sequence,
                        const_delay: self.const_delay,
                        resv0: 0,
                    }),
                };

                self.next_peer_sequence = body.initial_sequence;

                self.packet_queue.push_back(resp);
                self.state = ConnectionState::Handshake(HandshakeState::Agreement);
                return Poll::Ready(Ok(()));
            }
            HandshakeState::Agreement if M::IS_CONNECT => {
                if body.kind != HandshakeType::AGREEMENT {
                    tracing::info!("abort: expected AGREEMENT, but got {:?}", body.kind);
                    self.abort();
                    return Poll::Ready(Ok(()));
                }

                if self.next_peer_sequence != body.initial_sequence {
                    tracing::warn!(
                        "peer changed initial sequence between HELLO ({}) and AGREEMENT ({})",
                        self.next_peer_sequence.to_bits(),
                        body.initial_sequence.to_bits(),
                    );
                }

                self.state = ConnectionState::Connected;

                self.writer
                    .try_send(Message::Control(ControlMessage::Connected()))
                    .unwrap();
            }
            // Listen mode
            HandshakeState::Hello if M::IS_LISTEN => {
                if body.kind != HandshakeType::HELLO {
                    tracing::info!("reject: expected HELLO, but got {:?}", body.kind);
                    self.reject(HandshakeType::REJ_ROGUE);
                    return Poll::Ready(Ok(()));
                }

                let initial_sequence = create_initial_sequence(self.local_addr, self.remote_addr);
                self.next_local_sequence = initial_sequence;

                // Send HELLO
                let resp = Packet {
                    header: Header {
                        packet_type: PacketType::HANDSHAKE,
                        sequence: Sequence::default(),
                        control_frame: ControlFrame(0),
                        flags: Flags::new(),
                    },
                    body: PacketBody::Handshake(Handshake {
                        version: 0,
                        kind: HandshakeType::HELLO,
                        flags: HandshakeFlags::default(),
                        mtu: 1500,
                        flow_window: 8192,
                        initial_sequence,
                        const_delay: self.const_delay,
                        resv0: 0,
                    }),
                };

                self.next_peer_sequence = body.initial_sequence;

                self.packet_queue.push_back(resp);
                self.state = ConnectionState::Handshake(HandshakeState::Agreement);
                return Poll::Ready(Ok(()));
            }
            HandshakeState::Agreement if M::IS_LISTEN => {
                if body.kind != HandshakeType::AGREEMENT {
                    tracing::info!("reject: expected AGREEMENT, but got {:?}", body.kind);
                    self.reject(HandshakeType::REJ_ROGUE);
                    return Poll::Ready(Ok(()));
                }

                if self.next_peer_sequence != body.initial_sequence {
                    tracing::warn!(
                        "peer changed initial sequence between HELLO ({}) and AGREEMENT ({})",
                        self.next_peer_sequence.to_bits(),
                        body.initial_sequence.to_bits(),
                    );
                }

                let resp = Packet {
                    header: Header {
                        packet_type: PacketType::HANDSHAKE,
                        sequence: Sequence::default(),
                        control_frame: ControlFrame(0),
                        flags: Flags::new(),
                    },
                    body: PacketBody::Handshake(Handshake {
                        version: 0,
                        kind: HandshakeType::AGREEMENT,
                        flags: HandshakeFlags::default(),
                        mtu: 1500,
                        flow_window: 8192,
                        initial_sequence: self.next_local_sequence,
                        const_delay: self.const_delay,
                        resv0: 0,
                    }),
                };

                self.writer
                    .try_send(Message::Control(ControlMessage::Connected()))
                    .unwrap();

                self.packet_queue.push_back(resp);
                self.state = ConnectionState::Connected;
                return Poll::Ready(Ok(()));
            }
            // `M` is configured incorrectly. It must be either `IS_LISTEN` OR `IS_CONNECT`.
            HandshakeState::Hello | HandshakeState::Agreement => unreachable!(),
        }

        Poll::Pending
    }

    fn handle_shutdown(
        &mut self,
        _header: Header,
        body: Shutdown,
    ) -> Poll<Result<(), Error<S::Error>>> {
        // The shutdown packet is the confirmation of our request.
        // The connection is now considred dead.
        if self.state == ConnectionState::Shutdown {
            self.state = ConnectionState::Closing;
            return Poll::Ready(Ok(()));
        }

        // The shutdown was initialized from the remote peer.

        // If the connection active we need to notify that the player left.
        if M::IS_LISTEN && self.state == ConnectionState::Connected {
            // It is possible that both sides hang up simultaneously,
            // at which point this will return `Err(..)`.
            let _ = self
                .writer
                .try_send(Message::Control(ControlMessage::Disconnected));
        }

        let packet = Packet {
            header: Header {
                packet_type: PacketType::SHUTDOWN,
                sequence: Sequence::new(0),
                control_frame: ControlFrame(0),
                flags: Flags::new(),
            },
            body: PacketBody::Shutdown(Shutdown {
                reason: body.reason,
            }),
        };

        self.packet_queue.push_back(packet);
        self.state = ConnectionState::Closing;
        Poll::Ready(Ok(()))
    }

    fn handle_ack(&mut self, header: Header, body: Ack) -> Poll<Result<(), Error<S::Error>>> {
        if !matches!(self.state, ConnectionState::Connected) {
            return Poll::Pending;
        }

        let sequence = body.sequence;

        // Convert back to local control frame.
        let control_frame = header.control_frame + self.start_control_frame;

        tracing::debug!("got ACK for {:?}", control_frame);
        self.writer
            .try_send(Message::Control(ControlMessage::Ack(control_frame)))
            .unwrap();
        if let Some(id) = self.message_out.remove(&sequence) {
            self.writer
                .try_send(Message::Control(ControlMessage::Acknowledge(
                    id,
                    control_frame,
                )))
                .unwrap();
        }

        // Respond with ACKACK
        let packet = Packet {
            header: Header {
                packet_type: PacketType::ACKACK,
                sequence: Sequence::new(0),
                control_frame: ControlFrame(0),
                flags: Flags::new(),
            },
            body: PacketBody::AckAck(AckAck {
                ack_sequence: body.ack_sequence,
            }),
        };

        self.packet_queue.push_back(packet);
        self.state = ConnectionState::Connected;
        Poll::Ready(Ok(()))
    }

    fn handle_ackack(
        &mut self,
        _header: Header,
        body: AckAck,
    ) -> Poll<Result<(), Error<S::Error>>> {
        tracing::trace!("got ACKACK for {:?}", body.ack_sequence);

        if let Some(ts) = self.ack_time_list.remove(body.ack_sequence) {
            tracing::trace!("RTT is {}us", ts.elapsed().as_micros());
            self.rtt.lock().update(ts.elapsed().as_micros() as u32);
        }

        Poll::Pending
    }

    fn handle_nak(&mut self, _header: Header, body: Nak) -> Poll<Result<(), Error<S::Error>>> {
        let mut start = body.sequences.start;
        let end = body.sequences.end;

        // This is an inclusive range.
        while start <= end {
            if let Some(packet) = self.inflight_packets.get(start) {
                self.packet_queue.push_back(packet.clone());
            }

            start += 1;
        }

        Poll::Ready(Ok(()))
    }

    fn prepare_connect(&mut self) {
        let initial_sequence = create_initial_sequence(self.local_addr, self.remote_addr);
        self.next_local_sequence = initial_sequence;

        let packet = Packet {
            header: Header {
                packet_type: PacketType::HANDSHAKE,
                sequence: Sequence::default(),
                control_frame: ControlFrame(0),
                flags: Flags::new(),
            },
            body: PacketBody::Handshake(Handshake {
                version: 0,
                kind: HandshakeType::HELLO,
                flags: HandshakeFlags::default(),
                mtu: 1500,
                flow_window: 8192,
                initial_sequence,
                const_delay: self.const_delay,
                resv0: 0,
            }),
        };

        self.packet_queue.push_back(packet);
        self.state = ConnectionState::Handshake(HandshakeState::Hello);
        self.init_write().unwrap();
    }

    fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error<S::Error>>> {
        // `tokio::time::Interval::poll_tick` does no longer register the waker if
        // the interval yields on the first call. We need to register the waker to
        // advance the state machine correctly. We expect that at some point `poll_tick`
        // should yield `Pending` and the waker is registered, but handling multiple ticks
        // is not actually necessary.
        // FIXME: It might make sense to replace this with a custom time driver.
        while let Poll::Ready(tick) = self.interval.poll_tick(cx) {
            if self.last_time.elapsed() >= Duration::from_secs(15) {
                tracing::info!("closing connection due to timeout");

                self.shutdown();
                return Poll::Ready(Err(Error::Timeout));
            }

            // Send periodic ACKs while connected.
            if self.state == ConnectionState::Connected && tick.is_ack() {
                // FIXME: This seems kinda awkward.
                // What should we actually send? The last received sequence,
                // last recevied sequence without NAKs, or the last sequence
                // + 1? The current connection impl only acknowledges messages
                // if the ACK contains the exact sequence of the last packet,
                // so if we actually `next_peer_sequence`, but previous packet
                // was lost, the peer will still acknowledge the message.
                // This is bad.
                let sequence = self.next_peer_sequence - 1;

                let ack_sequence = self.next_ack_sequence.fetch_next();

                let packet = Packet {
                    header: Header {
                        packet_type: PacketType::ACK,
                        sequence: Sequence::new(0),
                        control_frame: self.last_cf,
                        flags: Flags::new(),
                    },
                    body: PacketBody::Ack(Ack {
                        sequence,
                        ack_sequence,
                    }),
                };

                tracing::trace!("send ACK for {:?}", ack_sequence);
                self.ack_time_list.insert(ack_sequence);

                self.packet_queue.push_back(packet);
                return Poll::Ready(Ok(()));
            }
        }

        Poll::Pending
    }

    fn reject(&mut self, reason: HandshakeType) {
        // Don't accidently send a non-rejection.
        debug_assert!(reason.is_rejection());

        let resp = Packet {
            header: Header {
                packet_type: PacketType::HANDSHAKE,
                sequence: Sequence::new(0),
                control_frame: ControlFrame(0),
                flags: Flags::new(),
            },
            body: PacketBody::Handshake(Handshake {
                version: 0,
                kind: reason,
                flags: HandshakeFlags::default(),
                mtu: 1500,
                flow_window: 8192,
                initial_sequence: Sequence::new(0),
                const_delay: 0,
                resv0: 0,
            }),
        };

        self.packet_queue.push_back(resp);
        self.state = ConnectionState::Closing;
    }

    /// Closes the connection without doing a shutdown process.
    fn abort(&mut self) {
        // If the connection active we need to notify that the player left.
        if self.state == ConnectionState::Connected {
            self.writer
                .try_send(Message::Control(ControlMessage::Disconnected))
                .unwrap();
        }

        self.state = ConnectionState::Closing;
    }

    /// Initialize a connection shutdown.
    fn shutdown(&mut self) {
        // If the connection active we need to notify that the player left.
        if M::IS_LISTEN && self.state == ConnectionState::Connected {
            let _ = self
                .writer
                .try_send(Message::Control(ControlMessage::Disconnected));
        }

        let packet = Packet {
            header: Header {
                packet_type: PacketType::SHUTDOWN,
                sequence: Sequence::new(0),
                control_frame: ControlFrame(0),
                flags: Flags::new(),
            },
            body: PacketBody::Shutdown(Shutdown {
                reason: ShutdownReason::CLOSE,
            }),
        };

        // Wait for the remote peer to confirm the shutdown
        // before closing the stream.
        self.packet_queue.push_back(packet);
        self.state = ConnectionState::Shutdown;
    }

    fn poll_closing(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Error<<S as Sink<Packet>>::Error>>> {
        let res = ready!(self.stream.poll_close_unpin(cx));
        self.state = ConnectionState::Closed;

        Poll::Ready(res.map_err(Error::Stream))
    }
}

impl<S, M> Future for Connection<S, M>
where
    S: ConnectionStream,
    S::Error: std::error::Error,
    M: ConnectionMode,
{
    type Output = Result<(), Error<S::Error>>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let _span = trace_span!("Connection::poll").entered();

        loop {
            if self.is_writing {
                match self.poll_write(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Ok(())) => (),
                    Poll::Ready(Err(err)) => return Poll::Ready(Err(err.into())),
                }
            }

            match self.state {
                ConnectionState::Connected
                | ConnectionState::Handshake(_)
                | ConnectionState::Shutdown => match self.poll_read(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                    Poll::Ready(Ok(())) => (),
                },
                ConnectionState::Closing => return self.poll_closing(cx),
                ConnectionState::Closed => return Poll::Ready(Ok(())),
            }
        }
    }
}

impl<S, M> FusedFuture for Connection<S, M>
where
    S: ConnectionStream,
    S::Error: std::error::Error,
    M: ConnectionMode,
{
    #[inline]
    fn is_terminated(&self) -> bool {
        matches!(self.state, ConnectionState::Closed)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum ConnectionState {
    Handshake(HandshakeState),
    Connected,
    Shutdown,
    Closing,
    Closed,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum HandshakeState {
    ///
    /// If [`ConnectionMode::Connect`]: Hello sent, waiting for HELLO from server.
    /// If [`ConnectionMode::Listen`]: Waiting for HELLO from client.
    Hello,
    Agreement,
}

#[derive(Debug)]
pub struct ConnectionHandle {
    chan_out: mpsc::Sender<Message>,
    rx: Mutex<mpsc::Receiver<Message>>,
    rtt: Arc<Mutex<Rtt>>,
}

impl ConnectionHandle {
    /// Client must set id.
    pub fn send(&self, msg: DataMessage) -> Result<(), TrySendError<DataMessage>> {
        match self.chan_out.try_send(Message::Data(msg)) {
            Ok(()) => Ok(()),
            Err(err) => match err {
                TrySendError::Full(msg) => match msg {
                    Message::Data(msg) => Err(TrySendError::Full(msg)),
                    _ => unreachable!(),
                },
                TrySendError::Closed(msg) => match msg {
                    Message::Data(msg) => Err(TrySendError::Closed(msg)),
                    _ => unreachable!(),
                },
            },
        }
    }

    /// Acknowledges the use of the message.
    pub fn acknowledge(&self, id: MessageId, cf: ControlFrame) {
        self.chan_out
            .try_send(Message::Control(ControlMessage::Acknowledge(id, cf)))
            .unwrap();
    }

    /// Sets the last finished [`ControlFrame`] to the value.
    pub fn set_cf(&self, cf: ControlFrame) {
        self.chan_out
            .try_send(Message::Control(ControlMessage::Ack(cf)))
            .unwrap();
    }

    pub fn recv(&self) -> Option<Message> {
        let mut r = self.rx.lock();
        r.try_recv().ok()
    }

    pub fn is_connected(&self) -> bool {
        !self.rx.lock().is_closed()
    }

    pub fn rtt(&self) -> Duration {
        Duration::from_micros(self.rtt.lock().rtt.into())
    }
}

pub struct ConnectionKey {}

pub trait ConnectionMode {
    const IS_CONNECT: bool;
    const IS_LISTEN: bool;
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Connect;

impl ConnectionMode for Connect {
    const IS_CONNECT: bool = true;
    const IS_LISTEN: bool = false;
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Listen;

impl ConnectionMode for Listen {
    const IS_CONNECT: bool = false;
    const IS_LISTEN: bool = true;
}

#[derive(Debug)]
struct TickInterval {
    interval: tokio::time::Interval,
    tick: u16,
}

impl TickInterval {
    fn new() -> Self {
        let mut interval = tokio::time::interval(Duration::from_millis(50));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Self { interval, tick: 0 }
    }

    fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Tick> {
        self.interval.poll_tick(cx).map(|_| {
            let tick = Tick { tick: self.tick };
            self.tick += 1;
            tick
        })
    }
}

#[derive(Copy, Clone, Debug)]
struct Tick {
    tick: u16,
}

impl Tick {
    fn is_ack(&self) -> bool {
        (self.tick & 10) == 0
    }
}

#[derive(Clone, Debug)]
struct InflightPackets {
    packets: Box<[Option<Packet>]>,
}

impl InflightPackets {
    fn new(size: usize) -> Self {
        let packets = vec![None; size];

        Self {
            packets: packets.into_boxed_slice(),
        }
    }

    fn insert(&mut self, packet: Packet) {
        let index = packet.header.sequence.to_bits() as usize % self.packets.len();
        self.packets[index] = Some(packet);
    }

    fn get(&self, seq: Sequence) -> Option<&Packet> {
        let index = seq.to_bits() as usize % self.packets.len();

        // The index always exists.
        if let Some(packet) = self.packets.get(index).unwrap() {
            if packet.header.sequence == seq {
                return Some(packet);
            }
        }

        None
    }
}

impl Default for InflightPackets {
    fn default() -> Self {
        Self::new(8192)
    }
}

/// Generates a new initial [`Sequence`] for the given tuple.
fn create_initial_sequence(local_addr: SocketAddr, remote_addr: SocketAddr) -> Sequence {
    // The initial sequence number (ISN) should be imposible to predict from the outside
    // and the same socket tuple should be guaranteed to have a unique sequence that does
    // not collide with previous connections with the same tuple.
    // See Transmission Control Protocol (TCP) and Defending against Sequence Number Attacks:
    // https://datatracker.ietf.org/doc/html/rfc9293#name-initial-sequence-number-sel
    // https://datatracker.ietf.org/doc/html/rfc6528#section-3

    // We use the following formula:
    // `ISN = M + SHA256(localip, localport, remoteip, remoteport, secretkey)`
    // `secretkey` is newly generated whenever the application is restarted.
    // This is sufficient as mentioned by RFC 6528.

    static CLOCK: LazyLock<Instant> = LazyLock::new(Instant::now);
    static SECRET: LazyLock<[u8; 16]> = LazyLock::new(|| OsRng.gen());

    // RFC 6528 recommends using a 4 microsecond clock.
    // Take the least signigicant bits from the clock, effectively making
    // the clock wrap at `u32::MAX`.
    let timestamp = ((CLOCK.elapsed().as_micros() / 4) & u32::MAX as u128) as u32;

    let mut hasher = Sha256::new();
    match local_addr.ip() {
        IpAddr::V4(addr) => {
            hasher.update(addr.octets());
        }
        IpAddr::V6(addr) => {
            hasher.update(addr.octets());
        }
    }
    hasher.update(local_addr.port().to_ne_bytes());

    match remote_addr.ip() {
        IpAddr::V4(addr) => {
            hasher.update(addr.octets());
        }
        IpAddr::V6(addr) => {
            hasher.update(addr.octets());
        }
    }
    hasher.update(remote_addr.port().to_ne_bytes());

    hasher.update(&*SECRET);

    let hash = hasher.finalize();
    let hash: &[u8; 32] = hash.as_ref();
    let hash = u32::from_ne_bytes(hash[0..4].try_into().unwrap());

    // Only retain 31 bits since that is the length of our sequence
    // in the header.
    let bits = timestamp.wrapping_add(hash) & ((1 << 31) - 1);
    Sequence::new(bits)
}

fn fragment_frame(
    frame: &Frame,
    next_local_sequence: &mut Sequence,
    control_frame: ControlFrame,
    mss: u16,
    mut write_packet: impl FnMut(Packet),
) {
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();

    // If possible write the data in a single segment.
    if buf.len() <= mss as usize {
        let mut flags = Flags::new();
        flags.set_packet_position(PacketPosition::Single);

        let packet = Packet {
            header: Header {
                packet_type: PacketType::DATA,
                sequence: next_local_sequence.fetch_next(),
                control_frame,
                flags,
            },
            body: PacketBody::Data(buf),
        };

        write_packet(packet);
        return;
    }

    {
        let mut flags = Flags::new();
        flags.set_packet_position(PacketPosition::First);

        let packet = Packet {
            header: Header {
                packet_type: PacketType::DATA,
                sequence: next_local_sequence.fetch_next(),
                control_frame,
                flags,
            },
            body: PacketBody::Data(buf[..mss as usize].to_vec()),
        };

        write_packet(packet);
    }

    let mut next_byte = mss as usize;
    while next_byte < buf.len() - (mss as usize) {
        let mut flags = Flags::new();
        flags.set_packet_position(PacketPosition::Middle);

        let packet = Packet {
            header: Header {
                packet_type: PacketType::DATA,
                sequence: next_local_sequence.fetch_next(),
                control_frame,
                flags,
            },
            body: PacketBody::Data(buf[next_byte..next_byte + mss as usize].to_vec()),
        };

        write_packet(packet);
        next_byte += mss as usize;
    }

    let mut flags = Flags::new();
    flags.set_packet_position(PacketPosition::Last);

    let packet = Packet {
        header: Header {
            packet_type: PacketType::DATA,
            sequence: next_local_sequence.fetch_next(),
            control_frame,
            flags,
        },
        body: PacketBody::Data(buf[next_byte..].to_vec()),
    };

    write_packet(packet);
}

#[derive(Clone, Debug)]
struct AckTimeList {
    inner: Box<[Option<Instant>]>,
}

impl AckTimeList {
    fn new() -> Self {
        let vec = vec![None; 8192];

        Self {
            inner: vec.into_boxed_slice(),
        }
    }

    fn insert(&mut self, seq: Sequence) {
        let index = seq.to_bits() as usize % self.inner.len();
        self.inner[index] = Some(Instant::now());
    }

    fn remove(&mut self, seq: Sequence) -> Option<Instant> {
        let index = seq.to_bits() as usize % self.inner.len();
        self.inner[index]
    }
}

#[derive(Copy, Clone, Debug)]
struct Rtt {
    /// Rtt in micros
    rtt: u32,
}

impl Rtt {
    fn new() -> Self {
        Self { rtt: 100_000 }
    }

    fn update(&mut self, rtt: u32) {
        self.rtt = ((self.rtt as f32 * 0.9) + (rtt as f32 * 0.1)) as u32;
    }
}

impl Default for Rtt {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use game_common::components::components::RawComponent;
    use game_common::net::ServerEntity;
    use game_common::record::RecordReference;
    use game_common::world::control_frame::ControlFrame;

    use crate::proto::components::ComponentAdd;
    use crate::proto::sequence::Sequence;
    use crate::proto::{
        Encode, Flags, Frame, Header, Packet, PacketBody, PacketPosition, PacketType,
    };

    use super::{fragment_frame, InflightPackets};

    fn create_packet(seq: Sequence) -> Packet {
        Packet {
            header: Header {
                packet_type: PacketType::DATA,
                sequence: seq,
                control_frame: ControlFrame::new(),
                flags: Flags::new(),
            },
            body: PacketBody::Data(vec![]),
        }
    }

    #[test]
    fn inflight_packets() {
        let mut packets = InflightPackets::default();
        for seq in 0..1024 {
            packets.insert(create_packet(Sequence::new(seq)));
        }

        for seq in 0..1024 {
            assert!(packets.get(Sequence::new(seq)).is_some());
        }
    }

    #[test]
    fn inflight_packets_overflow() {
        let size = 8192;

        let mut packets = InflightPackets::new(size);
        for seq in 0..size as u32 * 2 {
            packets.insert(create_packet(Sequence::new(seq)));
        }

        // First size dropped.
        for seq in 0..size as u32 {
            assert!(packets.get(Sequence::new(seq)).is_none());
        }

        for seq in size as u32..size as u32 * 2 {
            assert!(packets.get(Sequence::new(seq)).is_some());
        }
    }

    /// Returns a `Frame` of at least `len` bytes.
    fn create_big_frame(len: usize) -> Frame {
        Frame::EntityComponentAdd(ComponentAdd {
            entity: ServerEntity(0),
            component_id: RecordReference::STUB,
            component: RawComponent::new(vec![0; len], vec![]),
        })
    }

    #[test]
    fn fragment_frame_single() {
        let frame = create_big_frame(512);
        let mss = 1024;
        let mut sequence = Sequence::new(0);

        let mut packets = vec![];
        fragment_frame(&frame, &mut sequence, ControlFrame(0), mss, |packet| {
            packets.push(packet);
        });

        assert_eq!(packets.len(), 1);
        let pkt = &packets[0];
        assert_eq!(pkt.header.flags.packet_position(), PacketPosition::Single);
        assert_eq!(pkt.header.sequence, Sequence::new(0));
        assert!(pkt.body.as_data().unwrap().len() <= mss as usize);
    }

    #[test]
    fn fragment_frame_big() {
        let frame = create_big_frame(u16::MAX as usize);
        let mss = 1024;
        let mut sequence = Sequence::new(0);

        let mut buf = Vec::new();
        frame.encode(&mut buf).unwrap();
        let total_size = buf.len();

        let mut packets = vec![];
        fragment_frame(&frame, &mut sequence, ControlFrame(0), mss, |packet| {
            packets.push(packet);
        });

        let mut sum = 0;
        let mut seq = Sequence::new(0);
        for (index, pkt) in packets.iter().enumerate() {
            if index == 0 {
                assert_eq!(pkt.header.flags.packet_position(), PacketPosition::First);
            } else if index == packets.len() - 1 {
                assert_eq!(pkt.header.flags.packet_position(), PacketPosition::Last);
            } else {
                assert_eq!(pkt.header.flags.packet_position(), PacketPosition::Middle);
            }

            assert_eq!(pkt.header.sequence, seq);
            assert!(pkt.body.as_data().unwrap().len() <= mss as usize);

            seq.fetch_next();
            sum += pkt.body.as_data().unwrap().len();
        }

        assert_eq!(sum, total_size);
    }
}
