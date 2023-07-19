use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::marker::PhantomData;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::task::{ready, Context, Poll};
use std::time::{Duration, Instant};

use futures::FutureExt;
use game_common::world::control_frame::ControlFrame;
use game_common::world::gen::flat::FlatGenerator;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;

use crate::buffer::FrameBuffer;
use crate::entity::{pack_command, unpack_command};
use crate::proto::ack::{Ack, AckAck, Nak};
use crate::proto::handshake::{Handshake, HandshakeFlags, HandshakeType};
use crate::proto::sequence::Sequence;
use crate::proto::shutdown::{Shutdown, ShutdownReason};
use crate::proto::{
    Decode, Encode, Flags, Frame, Header, Packet, PacketBody, PacketPosition, PacketType,
    SequenceRange,
};
use crate::request::Request;
use crate::snapshot::{
    Command, CommandId, CommandQueue, Connected, ConnectionMessage, Response, Status,
};
use crate::socket::Socket;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct Error(#[from] ErrorKind);

#[derive(Debug, Error)]
enum ErrorKind {
    #[error("connection refused")]
    ConnectionRefused,
    #[error("timed out")]
    TimedOut,
    #[error("peer shutdown")]
    PeerShutdown,
}

static CONNECTION_ID: AtomicU32 = AtomicU32::new(0);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConnectionId(pub u32);

impl ConnectionId {
    #[inline]
    pub fn new() -> Self {
        let id = CONNECTION_ID.fetch_add(1, Ordering::Relaxed);
        Self(id)
    }
}

impl Default for ConnectionId {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

pub struct Connection<M>
where
    M: ConnectionMode,
{
    pub id: ConnectionId,
    /// Input stream from the socket
    socket_stream: mpsc::Receiver<Packet>,
    socket: Arc<Socket>,

    /// Direction (from self)
    local_stream: mpsc::Receiver<ConnectionMessage>,

    state: ConnectionState,

    queue: CommandQueue,
    peer: SocketAddr,
    frame_queue: FrameQueue,
    write: Option<WriteRequest>,
    interval: TickInterval,
    last_time: Instant,
    start_time: Instant,

    next_local_sequence: Sequence,
    next_peer_sequence: Sequence,

    next_ack_sequence: Sequence,

    commands: Commands,
    buffer: FrameBuffer,
    write_queue: WriteQueue,
    ack_list: AckList,

    /// List of lost packets from the peer.
    loss_list: LossList,

    /// Packets that have been sent and are buffered until an ACK is received for them.
    inflight_packets: InflightPackets,

    start_control_frame: ControlFrame,
    peer_start_control_frame: ControlFrame,

    /// Local constant buffer in control frames.
    const_delay: u16,

    _mode: PhantomData<fn() -> M>,

    #[cfg(debug_assertions)]
    debug_validator: crate::validator::DebugValidator,

    max_data_size: u16,

    reassembly_buffer: ReassemblyBuffer,
}

impl<M> Connection<M>
where
    M: ConnectionMode,
{
    pub fn new(
        peer: SocketAddr,
        queue: CommandQueue,
        socket: Arc<Socket>,
        control_frame: ControlFrame,
        const_delay: ControlFrame,
    ) -> (Self, ConnectionHandle) {
        let id = ConnectionId::new();

        let (tx, rx) = mpsc::channel(4096);
        let (out_tx, out_rx) = mpsc::channel(4096);

        let mut conn = Self {
            id,
            socket_stream: rx,
            socket,
            state: ConnectionState::Handshake(HandshakeState::Hello),
            local_stream: out_rx,
            queue,
            peer,
            frame_queue: FrameQueue::new(),
            write: None,
            interval: TickInterval::new(),
            last_time: Instant::now(),
            next_local_sequence: Sequence::default(),
            next_ack_sequence: Sequence::default(),
            start_time: Instant::now(),
            next_peer_sequence: Sequence::default(),
            commands: Commands {
                cmds: Default::default(),
            },
            buffer: FrameBuffer::new(),
            write_queue: WriteQueue::new(),
            ack_list: AckList::default(),
            loss_list: LossList::new(),
            inflight_packets: InflightPackets::new(8192),

            start_control_frame: control_frame,
            peer_start_control_frame: ControlFrame::default(),

            _mode: PhantomData,

            const_delay: const_delay.0 as u16,

            max_data_size: 512,
            reassembly_buffer: ReassemblyBuffer::default(),

            #[cfg(debug_assertions)]
            debug_validator: crate::validator::DebugValidator::new(),
        };

        if M::IS_CONNECT {
            conn.prepare_connect();
        }

        (
            conn,
            ConnectionHandle {
                id,
                tx,
                chan_out: out_tx,
                next_id: Arc::new(AtomicU32::new(0)),
            },
        )
    }

    fn poll_read(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ErrorKind>> {
        tracing::trace!("Connection.poll_read");

        // Flush the send buffer before reading any packets.
        if !self.write_queue.is_empty() {
            self.init_write(self.state);
            return Poll::Ready(Ok(()));
        }

        while let Poll::Ready(packet) = self.socket_stream.poll_recv(cx) {
            tracing::debug!("recv {:?}", packet);

            self.last_time = Instant::now();

            let Some(packet) = packet else {
                return self.abort();
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

        while let Poll::Ready(msg) = self.local_stream.poll_recv(cx) {
            let Some(msg) = msg else {
                return self.shutdown();
            };

            match &msg.command {
                // The game loop has processed the commands.
                // We can now acknowledge that we have processed the commands.
                Command::ReceivedCommands(ids) => {
                    for resp in ids {
                        let seq = self.ack_list.list.remove(&resp.id).unwrap();
                        // The last sequence acknowledged by the game loop.
                        if seq < self.ack_list.ack_seq {
                            panic!(
                                "ack sequence went backwards: {:?} < {:?}",
                                seq, self.ack_list.ack_seq
                            );
                        }
                        self.ack_list.ack_seq = seq;
                    }

                    continue;
                }
                _ => (),
            }

            let msgid = msg.id.unwrap();

            let frame = match pack_command(&msg.command) {
                Some(frame) => frame,
                None => {
                    tracing::trace!("skipping command: {:?}", msg.command,);

                    continue;
                }
            };

            self.frame_queue.push(frame, msgid, msg.control_frame);
        }

        Poll::Pending
    }

    fn write_snapshot(&mut self) {
        // Merge FrameQueue into FrameBuffer, compact then send.

        while let Some((frame, id, cf)) = self.frame_queue.pop() {
            self.push_data_frame(&frame, cf);
        }

        self.buffer.clear();
    }

    fn init_write(&mut self, state: ConnectionState) {
        let socket = self.socket.clone();
        let peer = self.peer;

        let packet = self.write_queue.pop().unwrap();

        self.write = Some(WriteRequest {
            future: Box::pin(async move {
                let mut buf = Vec::with_capacity(1500);
                packet.encode(&mut buf).unwrap();
                if let Err(err) = socket.send_to(&buf, peer).await {
                    tracing::error!("failed to send packet: {}", err);
                }
            }),
            state,
        });
    }

    fn poll_write(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        tracing::trace!("Connection.poll_write");

        #[cfg(debug_assertions)]
        assert!(self.write.is_some());

        let Some(req) = &mut self.write else {
            unreachable!();
        };

        match req.future.poll_unpin(cx) {
            Poll::Ready(_) => {
                self.state = req.state;
                self.write = None;
                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
        }
    }

    ///
    /// Returns `Poll::Ready` on state change.
    fn handle_packet(&mut self, packet: Packet) -> Poll<()> {
        match packet.body {
            PacketBody::Handshake(body) => self.handle_handshake(packet.header, body),
            PacketBody::Shutdown(body) => self.handle_shutdown(packet.header, body),
            PacketBody::Ack(body) => self.handle_ack(packet.header, body),
            PacketBody::AckAck(body) => self.handle_ackack(packet.header, body),
            PacketBody::Nak(body) => self.handle_nak(packet.header, body),
            PacketBody::Data(body) => self.handle_data(packet.header, body),
        }
    }

    fn handle_data(&mut self, header: Header, body: Vec<u8>) -> Poll<()> {
        // Drop out-of-order or duplicates.
        if header.sequence < self.next_peer_sequence && !self.loss_list.remove(header.sequence) {
            tracing::warn!("dropping duplicate packet {:?}", header.sequence);
            return Poll::Ready(());
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
            self.loss_list.push(self.next_peer_sequence);
            self.next_peer_sequence += 1;
        }
        self.next_peer_sequence += 1;

        // Caught up to most recent packet.
        debug_assert_eq!(self.next_peer_sequence, header.sequence + 1);

        self.reassembly_buffer
            .insert(header.sequence, header.flags.packet_position(), body);

        while let Some(frame) = self.reassembly_buffer.pop() {
            #[cfg(debug_assertions)]
            self.debug_validator.push(header, &frame);

            let Some(cmd) = unpack_command(frame) else {
                tracing::debug!("failed to translate cmd");
                continue;
            };

            let msgid = self.ack_list.next_cmd_id;
            self.ack_list.next_cmd_id.0 += 1;

            self.ack_list.list.insert(msgid, header.sequence);

            // Convert back to local control frame.
            let control_frame =
                header.control_frame - (self.peer_start_control_frame - self.start_control_frame);

            self.queue.push(ConnectionMessage {
                id: Some(msgid),
                conn: self.id,
                control_frame,
                command: cmd,
            });
        }

        if let Some(nak) = nak {
            self.send_packet(nak, ConnectionState::Connected)
        } else {
            Poll::Ready(())
        }
    }

    fn handle_handshake(&mut self, header: Header, body: Handshake) -> Poll<()> {
        // Ignore if not in HS process.
        let ConnectionState::Handshake(state) = self.state else {
            return Poll::Ready(());
        };

        match state {
            // Connect mode
            HandshakeState::Hello if M::IS_CONNECT => {
                if body.kind != HandshakeType::HELLO {
                    tracing::info!("abort: expected HELLO, but got {:?}", body.kind);
                    self.abort();
                    return Poll::Ready(());
                }

                // Send AGREEMENT
                let resp = Packet {
                    header: Header {
                        packet_type: PacketType::HANDSHAKE,
                        sequence: Sequence::default(),
                        control_frame: self.start_control_frame,
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
                self.peer_start_control_frame = header.control_frame;

                self.ack_list.ack_seq = self.next_peer_sequence;

                return self
                    .send_packet(resp, ConnectionState::Handshake(HandshakeState::Agreement));
            }
            HandshakeState::Agreement if M::IS_CONNECT => {
                if body.kind != HandshakeType::AGREEMENT {
                    tracing::info!("abort: expected AGREEMENT, but got {:?}", body.kind);
                    self.abort();
                    return Poll::Ready(());
                }

                if self.next_peer_sequence != body.initial_sequence {
                    tracing::warn!(
                        "peer changed initial sequence between HELLO ({}) and AGREEMENT ({})",
                        self.next_peer_sequence.to_bits(),
                        body.initial_sequence.to_bits(),
                    );
                }

                self.state = ConnectionState::Connected;
                self.queue.push(ConnectionMessage {
                    id: None,
                    conn: self.id,
                    control_frame: ControlFrame(0),
                    command: Command::Connected(Connected {
                        peer_delay: ControlFrame(body.const_delay.into()),
                    }),
                });
            }
            // Listen mode
            HandshakeState::Hello if M::IS_LISTEN => {
                if body.kind != HandshakeType::HELLO {
                    tracing::info!("reject: expected HELLO, but got {:?}", body.kind);
                    return self.reject(HandshakeType::REJ_ROGUE);
                }

                let initial_sequence = create_initial_sequence();
                self.next_local_sequence = initial_sequence;

                // Send HELLO
                let resp = Packet {
                    header: Header {
                        packet_type: PacketType::HANDSHAKE,
                        sequence: Sequence::default(),
                        control_frame: self.start_control_frame,
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
                self.peer_start_control_frame = header.control_frame;

                self.ack_list.ack_seq = self.next_peer_sequence;

                return self
                    .send_packet(resp, ConnectionState::Handshake(HandshakeState::Agreement));
            }
            HandshakeState::Agreement if M::IS_LISTEN => {
                if body.kind != HandshakeType::AGREEMENT {
                    tracing::info!("reject: expected AGREEMENT, but got {:?}", body.kind);
                    return self.reject(HandshakeType::REJ_ROGUE);
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
                        control_frame: self.start_control_frame,
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

                // Signal the game that the player spawns.
                self.queue.push(ConnectionMessage {
                    id: None,
                    conn: self.id,
                    control_frame: ControlFrame(0),
                    command: Command::Connected(Connected {
                        peer_delay: ControlFrame(body.const_delay.into()),
                    }),
                });

                return self.send_packet(resp, ConnectionState::Connected);
            }
            // `M` is configured incorrectly. It must be either `IS_LISTEN` OR `IS_CONNECT`.
            HandshakeState::Hello | HandshakeState::Agreement => unreachable!(),
        }

        Poll::Pending
    }

    fn handle_shutdown(&mut self, header: Header, body: Shutdown) -> Poll<()> {
        let _ = self.shutdown();
        Poll::Ready(())
    }

    fn handle_ack(&mut self, header: Header, body: Ack) -> Poll<()> {
        let sequence = body.sequence;

        let ids = self.commands.remove(sequence);
        if !ids.is_empty() {
            self.queue.push(ConnectionMessage {
                id: None,
                conn: self.id,
                control_frame: ControlFrame(0),
                command: Command::ReceivedCommands(
                    ids.into_iter()
                        .map(|id| Response {
                            id,
                            status: Status::Received,
                        })
                        .collect(),
                ),
            });
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

        self.send_packet(packet, ConnectionState::Connected)
    }

    fn handle_ackack(&mut self, header: Header, body: AckAck) -> Poll<()> {
        Poll::Pending
    }

    fn handle_nak(&mut self, header: Header, body: Nak) -> Poll<()> {
        let mut start = body.sequences.start;
        let end = body.sequences.end;

        // This is an inclusive range.
        while start <= end {
            if let Some(packet) = self.inflight_packets.get(start) {
                self.write_queue.push(packet.clone());
            }

            start += 1;
        }

        Poll::Ready(())
    }

    fn prepare_connect(&mut self) {
        let initial_sequence = create_initial_sequence();
        self.next_local_sequence = initial_sequence;

        let packet = Packet {
            header: Header {
                packet_type: PacketType::HANDSHAKE,
                sequence: Sequence::default(),
                control_frame: self.start_control_frame,
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

        self.send_packet(packet, ConnectionState::Handshake(HandshakeState::Hello));
    }

    fn poll_tick(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), ErrorKind>> {
        let tick = ready!(self.interval.poll_tick(cx));

        if self.last_time.elapsed() >= Duration::from_secs(15) {
            tracing::info!("closing connection due to timeout");

            self.shutdown();
            return Poll::Ready(Err(ErrorKind::TimedOut));
        }

        // Send periodic ACKs while connected.
        if self.state == ConnectionState::Connected && tick.is_ack() {
            let ack_sequence = self.next_ack_sequence;
            self.next_ack_sequence += 1;

            let packet = Packet {
                header: Header {
                    packet_type: PacketType::ACK,
                    sequence: Sequence::new(0),
                    control_frame: ControlFrame(0),
                    flags: Flags::new(),
                },
                body: PacketBody::Ack(Ack {
                    sequence: self.ack_list.ack_seq,
                    ack_sequence,
                }),
            };

            self.send_packet(packet, ConnectionState::Connected);
            Poll::Ready(Ok(()))
        } else if self.state == ConnectionState::Connected && tick.is_fire() {
            self.write_snapshot();
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    fn reject(&mut self, reason: HandshakeType) -> Poll<()> {
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

        self.send_packet(resp, ConnectionState::Closed)
    }

    /// Closes the connection without doing a shutdown process.
    fn abort(&mut self) -> Poll<Result<(), ErrorKind>> {
        // If the connection active we need to notify that the player left.
        if self.state == ConnectionState::Connected {
            self.queue.push(ConnectionMessage {
                id: None,
                conn: self.id,
                control_frame: ControlFrame(0),
                command: Command::Disconnected,
            });
        }

        self.state = ConnectionState::Closed;

        Poll::Ready(Ok(()))
    }

    fn shutdown(&mut self) -> Poll<Result<(), ErrorKind>> {
        // If the connection active we need to notify that the player left.
        if M::IS_LISTEN && self.state == ConnectionState::Connected {
            self.queue.push(ConnectionMessage {
                id: None,
                conn: self.id,
                control_frame: ControlFrame(0),
                command: Command::Disconnected,
            });
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

        self.send_packet(packet, ConnectionState::Closed);
        Poll::Ready(Ok(()))
    }

    fn send_packet(&mut self, packet: Packet, state: ConnectionState) -> Poll<()> {
        let socket = self.socket.clone();
        let peer = self.peer;
        self.write = Some(WriteRequest {
            future: Box::pin(async move {
                let mut buf = Vec::with_capacity(1500);
                packet.encode(&mut buf).unwrap();
                if let Err(err) = socket.send_to(&buf, peer).await {
                    tracing::error!("Failed to send packet: {}", err);
                }
            }),
            state,
        });

        Poll::Ready(())
    }

    fn push_data_frame(&mut self, frame: &Frame, control_frame: ControlFrame) {
        fragment_frame(
            frame,
            &mut self.next_local_sequence,
            control_frame,
            self.max_data_size,
            |packet| {
                self.inflight_packets.insert(packet.clone());
                self.write_queue.push(packet);
            },
        );
    }
}

impl<M> Future for Connection<M>
where
    M: ConnectionMode,
{
    type Output = Result<(), Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        tracing::trace!("Connection.poll");

        loop {
            if self.write.is_some() {
                match self.poll_write(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(()) => (),
                }
            }

            match self.state {
                ConnectionState::Connected | ConnectionState::Handshake(_) => {
                    match self.poll_read(cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(err)) => return Poll::Ready(Err(Error(err))),
                        Poll::Ready(Ok(())) => (),
                    }
                }
                ConnectionState::Closed => return Poll::Ready(Ok(())),
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Sender {
    tx: mpsc::Sender<Frame>,
}

impl Sender {
    pub fn send<T>(&self, frame: T)
    where
        T: Into<Frame>,
    {
        let _ = self.tx.try_send(frame.into());
    }
}

struct WriteRequest {
    future: Pin<Box<(dyn Future<Output = ()> + Send + Sync + 'static)>>,
    /// The state to return to once done with writing.
    state: ConnectionState,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
enum ConnectionState {
    Handshake(HandshakeState),
    Connected,
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

#[derive(Clone, Debug)]
pub struct FrameQueue {
    queue: VecDeque<(Frame, CommandId, ControlFrame)>,
}

impl FrameQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, frame: Frame, id: CommandId, cf: ControlFrame) {
        self.queue.push_back((frame, id, cf));
    }

    pub fn pop(&mut self) -> Option<(Frame, CommandId, ControlFrame)> {
        self.queue.pop_front()
    }
}

impl Extend<(Frame, CommandId, ControlFrame)> for FrameQueue {
    fn extend<T: IntoIterator<Item = (Frame, CommandId, ControlFrame)>>(&mut self, iter: T) {
        self.queue.extend(iter);
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionHandle {
    pub id: ConnectionId,
    tx: mpsc::Sender<Packet>,
    chan_out: mpsc::Sender<ConnectionMessage>,
    next_id: Arc<AtomicU32>,
}

impl ConnectionHandle {
    pub async fn send(&self, packet: Packet) {
        self.tx.send(packet).await.unwrap();
    }

    pub fn send_cmd(&self, mut cmd: ConnectionMessage) -> CommandId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        cmd.id = Some(CommandId(id));

        self.chan_out.try_send(cmd);
        CommandId(id)
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

    fn is_fire(&self) -> bool {
        true
    }
}

struct Commands {
    cmds: HashMap<Sequence, Vec<CommandId>>,
}

impl Commands {
    /// Remove all commands where sequence  <= `seq`.
    fn remove(&mut self, seq: Sequence) -> Vec<CommandId> {
        let mut out = Vec::new();

        self.cmds.retain(|s, cmds| {
            if seq >= *s {
                out.extend(cmds.iter().copied());
                false
            } else {
                true
            }
        });

        out
    }
}

struct WriteQueue {
    packets: VecDeque<Packet>,
}

impl WriteQueue {
    #[inline]
    fn new() -> Self {
        Self {
            packets: VecDeque::new(),
        }
    }

    #[inline]
    fn len(&self) -> usize {
        self.packets.len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    fn push(&mut self, packet: Packet) {
        self.packets.push_back(packet);
    }

    #[inline]
    fn pop(&mut self) -> Option<Packet> {
        self.packets.pop_front()
    }
}

#[derive(Debug, Default)]
pub struct AckList {
    list: HashMap<CommandId, Sequence>,
    next_cmd_id: CommandId,
    ack_seq: Sequence,
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

/// A list of lost packets.
#[derive(Clone, Debug, Default)]
struct LossList {
    buffer: Vec<Sequence>,
}

impl LossList {
    fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    fn push(&mut self, seq: Sequence) {
        // The new sequence MUST be `> buffer.last()`.
        if cfg!(debug_assertions) {
            if let Some(n) = self.buffer.last() {
                if seq <= *n {
                    panic!(
                        "Tried to push {:?} to LossList with last sequence {:?}",
                        seq, n
                    );
                }
            }
        }

        self.buffer.push(seq);
    }

    /// Returns `true` if `seq` was removed.
    fn remove(&mut self, seq: Sequence) -> bool {
        let mut index = 0;
        while let Some(s) = self.buffer.get(index) {
            if *s == seq {
                self.buffer.remove(index);
                return true;
            }

            if *s > seq {
                return false;
            }

            index += 1;
        }

        false
    }
}

fn create_initial_sequence() -> Sequence {
    let bits = rand::random::<u32>() & ((1 << 31) - 1);
    Sequence::new(bits)
}

#[derive(Clone, Debug, Default)]
pub struct ReassemblyBuffer {
    /// Starting sequence => Bytes
    first_segments: HashMap<Sequence, Vec<u8>>,
    /// Middle and Last frames.
    other_segments: HashMap<Sequence, (Vec<u8>, PacketPosition)>,
    ready_frames: Vec<Frame>,
}

impl ReassemblyBuffer {
    pub fn insert(&mut self, seq: Sequence, packet_position: PacketPosition, buf: Vec<u8>) {
        match packet_position {
            PacketPosition::Single => {
                let frame = match Frame::decode(&buf[..]) {
                    Ok(frame) => frame,
                    Err(err) => {
                        tracing::debug!("failed to decode frame: {}", err);
                        return;
                    }
                };

                self.ready_frames.push(frame);
                return;
            }
            PacketPosition::First => {
                self.first_segments.insert(seq, buf);
            }
            PacketPosition::Middle | PacketPosition::Last => {
                self.other_segments.insert(seq, (buf, packet_position));

                let start_seq = self.find_starting_segment(seq);
                self.reassemble(start_seq);
            }
        }
    }

    fn find_starting_segment(&self, mut seq: Sequence) -> Sequence {
        while !self.first_segments.contains_key(&seq) {
            seq -= 1;
        }

        seq
    }

    fn reassemble(&mut self, mut start_seq: Sequence) {
        let mut buf = self.first_segments.get(&start_seq).unwrap().clone();

        let mut seq = start_seq + 1;
        let mut is_done = false;
        let mut end = start_seq;
        while !is_done {
            if let Some((next_buf, pos)) = self.other_segments.get(&seq) {
                buf.extend(next_buf);

                if pos.is_last() {
                    is_done = true;
                    end = start_seq;
                }
            } else {
                return;
            }

            seq += 1;
        }

        self.first_segments.remove(&start_seq);
        while start_seq != end {
            self.other_segments.remove(&seq);
            start_seq += 1;
        }

        let frame = match Frame::decode(&buf[..]) {
            Ok(frame) => frame,
            Err(err) => {
                tracing::debug!("failed to decode frame: {}", err);
                return;
            }
        };

        self.ready_frames.push(frame);
    }

    pub fn pop(&mut self) -> Option<Frame> {
        self.ready_frames.pop()
    }
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

    let mut bytes_written = mss as usize;
    while bytes_written < buf.len() - (mss as usize) {
        let mut flags = Flags::new();
        flags.set_packet_position(PacketPosition::Middle);

        let packet = Packet {
            header: Header {
                packet_type: PacketType::DATA,
                sequence: next_local_sequence.fetch_next(),
                control_frame,
                flags,
            },
            body: PacketBody::Data(
                buf[bytes_written + 1..bytes_written + 1 + mss as usize].to_vec(),
            ),
        };

        write_packet(packet);
        bytes_written += mss as usize;
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
        body: PacketBody::Data(buf[bytes_written + 1..].to_vec()),
    };

    write_packet(packet);
}

#[cfg(test)]
mod tests {
    use game_common::net::ServerEntity;
    use game_common::world::control_frame::ControlFrame;
    use game_common::world::entity::{EntityBody, Terrain};
    use game_common::world::terrain::{Heightmap, TerrainMesh};
    use game_common::world::CellId;
    use glam::{Quat, UVec2, Vec3};

    use crate::proto::sequence::Sequence;
    use crate::proto::{
        EntityCreate, Flags, Frame, Header, Packet, PacketBody, PacketPosition, PacketType,
    };

    use super::{fragment_frame, InflightPackets, LossList};

    #[test]
    fn loss_list_remove() {
        let mut input: Vec<u32> = (0..32).collect();

        let mut list = LossList::new();

        for seq in &input {
            list.push(Sequence::new(*seq));
        }

        while !input.is_empty() {
            // Always remove the middle element.
            let index = input.len() / 2;
            let value = input.remove(index);

            let res = list.remove(Sequence::new(value));
            assert!(res);
        }
    }

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

    fn create_terrain_frame(size: UVec2) -> Frame {
        let heightmap = Heightmap::from_u8(size, vec![0; size.x as usize * size.y as usize]);

        Frame::EntityCreate(EntityCreate {
            entity: ServerEntity(0),
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            data: EntityBody::Terrain(Terrain {
                mesh: TerrainMesh::new(CellId::ZERO, heightmap),
            }),
        })
    }

    #[test]
    fn fragment_frame_single() {
        let frame = create_terrain_frame(UVec2::new(1, 512));
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
        let frame = create_terrain_frame(UVec2::new(1, 65536));
        let mss = 1024;
        let mut sequence = Sequence::new(0);

        let mut packets = vec![];
        fragment_frame(&frame, &mut sequence, ControlFrame(0), mss, |packet| {
            packets.push(packet);
        });

        let mut seq = Sequence::new(0);
        for (index, pkt) in packets.iter().enumerate() {
            dbg!(index);
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
        }
    }
}
