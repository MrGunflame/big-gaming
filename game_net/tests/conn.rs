use game_common::world::control_frame::ControlFrame;
use game_net::conn::channel::ChannelStream;
use game_net::conn::{Connection, ConnectionHandle, Listen};
use game_net::proto::handshake::{Handshake, HandshakeFlags, HandshakeType};
use game_net::proto::sequence::Sequence;
use game_net::proto::shutdown::{Shutdown, ShutdownReason};
use game_net::proto::{Flags, Header, Packet, PacketBody, PacketType};
use tokio::sync::mpsc;

#[tokio::test]
async fn handshake() {
    let (tx, mut rx, handle) = create_listen_connection();

    let initial_sequence = Sequence::new(0x24945f);

    tx.try_send(Packet {
        header: Header {
            packet_type: PacketType::HANDSHAKE,
            sequence: Sequence::new(0),
            control_frame: ControlFrame(0),
            flags: Flags::new(),
        },
        body: PacketBody::Handshake(Handshake {
            version: 0,
            kind: HandshakeType::HELLO,
            flags: HandshakeFlags::NONE,
            mtu: 1500,
            flow_window: 8192,
            initial_sequence,
            const_delay: 0,
            resv0: 0,
        }),
    })
    .unwrap();

    let resp = rx.recv().await.unwrap();
    assert_eq!(resp.header.packet_type, PacketType::HANDSHAKE);
    let body = unwrap_handshake(resp);
    assert_eq!(body.version, 0);
    assert_eq!(body.kind, HandshakeType::HELLO);

    let server_isn = body.initial_sequence;

    tx.try_send(Packet {
        header: Header {
            packet_type: PacketType::HANDSHAKE,
            sequence: Sequence::new(0),
            control_frame: ControlFrame(0),
            flags: Flags::new(),
        },
        body: PacketBody::Handshake(Handshake {
            version: 0,
            kind: HandshakeType::AGREEMENT,
            flags: HandshakeFlags::NONE,
            mtu: 1500,
            flow_window: 8192,
            initial_sequence,
            const_delay: 0,
            resv0: 0,
        }),
    })
    .unwrap();

    let resp = rx.recv().await.unwrap();
    assert_eq!(resp.header.packet_type, PacketType::HANDSHAKE);
    let body = unwrap_handshake(resp);
    assert_eq!(body.version, 0);
    assert_eq!(body.kind, HandshakeType::AGREEMENT);

    assert_eq!(body.initial_sequence, server_isn);

    drop(handle);
}

fn create_listen_connection() -> (
    mpsc::Sender<Packet>,
    mpsc::Receiver<Packet>,
    ConnectionHandle,
) {
    let (tx0, rx0) = mpsc::channel(4096);
    let (tx1, rx1) = mpsc::channel(4096);
    let stream = ChannelStream::new(tx0, rx1);

    let (conn, handle) = Connection::<_, Listen>::new(stream, ControlFrame(0), ControlFrame(0));
    tokio::task::spawn(async move {
        conn.await.unwrap();
    });

    (tx1, rx0, handle)
}

async fn do_handshake(tx: &mut mpsc::Sender<Packet>, rx: &mut mpsc::Receiver<Packet>) {
    let initial_sequence = Sequence::new(0x24945f);

    tx.try_send(Packet {
        header: Header {
            packet_type: PacketType::HANDSHAKE,
            sequence: Sequence::new(0),
            control_frame: ControlFrame(0),
            flags: Flags::new(),
        },
        body: PacketBody::Handshake(Handshake {
            version: 0,
            kind: HandshakeType::HELLO,
            flags: HandshakeFlags::NONE,
            mtu: 1500,
            flow_window: 8192,
            initial_sequence,
            const_delay: 0,
            resv0: 0,
        }),
    })
    .unwrap();

    let resp = rx.recv().await.unwrap();
    assert_eq!(resp.header.packet_type, PacketType::HANDSHAKE);
    let body = unwrap_handshake(resp);
    assert_eq!(body.version, 0);
    assert_eq!(body.kind, HandshakeType::HELLO);

    let server_isn = body.initial_sequence;

    tx.try_send(Packet {
        header: Header {
            packet_type: PacketType::HANDSHAKE,
            sequence: Sequence::new(0),
            control_frame: ControlFrame(0),
            flags: Flags::new(),
        },
        body: PacketBody::Handshake(Handshake {
            version: 0,
            kind: HandshakeType::AGREEMENT,
            flags: HandshakeFlags::NONE,
            mtu: 1500,
            flow_window: 8192,
            initial_sequence,
            const_delay: 0,
            resv0: 0,
        }),
    })
    .unwrap();

    let resp = rx.recv().await.unwrap();
    assert_eq!(resp.header.packet_type, PacketType::HANDSHAKE);
    let body = unwrap_handshake(resp);
    assert_eq!(body.version, 0);
    assert_eq!(body.kind, HandshakeType::AGREEMENT);

    assert_eq!(body.initial_sequence, server_isn);
}

fn unwrap_handshake(packet: Packet) -> Handshake {
    match packet.body {
        PacketBody::Handshake(hs) => hs,
        _ => panic!(
            "invalid packet type: {:?}, expected handshake",
            packet.body.packet_type()
        ),
    }
}

#[tokio::test]
async fn shutdown() {
    let (mut tx, mut rx, handle) = create_listen_connection();
    do_handshake(&mut tx, &mut rx).await;

    let reason = ShutdownReason::CLOSE;

    tx.try_send(Packet {
        header: Header {
            packet_type: PacketType::SHUTDOWN,
            sequence: Sequence::new(0),
            control_frame: ControlFrame(0),
            flags: Flags::new(),
        },
        body: PacketBody::Shutdown(Shutdown { reason }),
    })
    .unwrap();

    while let Some(packet) = rx.recv().await {
        match packet.body {
            PacketBody::Shutdown(shutdown) => {
                assert_eq!(shutdown.reason, reason);
                break;
            }
            _ => (),
        }
    }

    drop(handle);
}
