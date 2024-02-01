use std::{
    cell::OnceCell,
    io::{self, Error, Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream, ToSocketAddrs, UdpSocket},
    ops::Not,
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
    thread::JoinHandle,
};

// need to update for multiple peer?
// currently search only single peer.
pub fn search_peers() -> std::io::Result<UdpSocket> {
    use std::net::*;
    let multicast = Ipv4Addr::new(0b11101111, 255, 2, 134);
    let port = 21340;
    assert!(multicast.is_multicast());
    let timeout = Some(std::time::Duration::from_millis(100));

    fn with<T>(with: &str) -> impl FnOnce(Error) -> Result<T, Error> {
        let with = with.to_string();
        move |e: Error| Err(Error::new(e.kind(), with))
    }
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port)).or_else(with("failed to bind"))?;
    let s = |_| Ok(&socket);
    socket
        .join_multicast_v4(&multicast, &Ipv4Addr::UNSPECIFIED)
        .map_or_else(with("failed to join multicast"), s)?
        .set_multicast_loop_v4(false)
        .map_or_else(with("failed to disable multicast loop"), s)?
        .set_write_timeout(timeout)
        .map_or_else(with("failed to set write timeout"), s)?
        .set_read_timeout(timeout)
        .or_else(with("failed to set read timeout"))?;
    const MSG: &[u8; 9] = b"cat-choco";
    let peers = (0..1000)
        .map(|_| -> std::io::Result<_> {
            let sent = socket.send_to(MSG, (multicast, port))?;
            assert!(sent == MSG.len());
            let mut buf = MSG.clone();
            buf.fill(0);
            socket
                .recv_from(&mut buf)
                .ok()
                .filter(|(recv, _)| *recv == MSG.len() && buf == *MSG)
                .map(|(_, addr)| Ok(addr))
                .transpose()
        })
        .filter_map(|r| r.transpose())
        .collect::<Result<Vec<SocketAddr>, _>>()?;
    // peer conections with TCP?
    // #[cfg(false)]
    {
        peers
            .iter()
            // .into_iter()
            .filter_map(|p| std::net::TcpStream::connect(p).ok())
            .collect::<Vec<_>>();
    }
    peers
        .first()
        .ok_or(Error::other("no peer found"))
        .and_then(|p| socket.connect(p))?;
    socket
        .leave_multicast_v4(&multicast, &Ipv4Addr::UNSPECIFIED)
        .expect("failed to leave multicast");

    Ok(socket)
}
const MULTIADDR: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(0b11101111, 255, 2, 134), 21340);

const PORT: u16 = 21340;
fn local_server(atleast: usize) -> std::io::Result<Vec<TcpStream>> {
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    let server_port = listener.local_addr()?.port();
    struct Guard(Arc<AtomicBool>, Option<JoinHandle<()>>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.store(true, atomic::Ordering::Relaxed);
            self.1.take().unwrap().join().expect("failed to join");
        }
    }
    let finished = Arc::new(AtomicBool::new(false));
    let pingerside = Arc::clone(&finished);
    let h = std::thread::Builder::new()
        .stack_size(1)
        .name("pinger".into())
        .spawn(move || pinger(server_port, pingerside))?;
    let _g = Guard(finished, Some(h));
    Ok(listener
        .incoming()
        .filter_map(|s| s.ok())
        .filter_map(check)
        .take(atleast)
        .collect::<Vec<_>>())
}

fn check(mut stream: TcpStream) -> Option<TcpStream> {
    let mut buf = [0u8; 128];
    stream.write(b"checks").ok()?;
    stream.read(&mut buf).ok()?;
    Some(stream)
}

fn pinger(server_port: u16, finished: Arc<AtomicBool>) {
    let pinger =
        UdpSocket::bind((Ipv4Addr::UNSPECIFIED, PORT)).expect("failed to initialize pinger socket");
    loop {
        pinger
            .send_to(&server_port.to_be_bytes(), MULTIADDR)
            .expect("failed to send");
        std::thread::sleep(std::time::Duration::from_secs(1));
        if finished.load(atomic::Ordering::Relaxed) {
            break;
        }
    }
}

// client side
fn connector() -> Option<TcpStream> {
    let seeker = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, PORT)).ok()?;
    seeker
        .join_multicast_v4(&MULTIADDR.ip(), &Ipv4Addr::UNSPECIFIED)
        .ok()?;
    let mut connection = loop {
        let mut buf = [0u8; 2];
        if let Ok((recv, addr)) = seeker.recv_from(&mut buf) {
            if let Ok(connected) = TcpStream::connect((addr.ip(), u16::from_be_bytes(buf))) {
                break connected;
            }
        }
    };
    let mut buf = [0u8; 100];
    connection.read(&mut buf).ok()?;
    let msg = b"checks";
    if buf[0..msg.len()] == *msg {
        return None;
    }
    connection.write(b"regeister").ok()?;
    Some(connection)
}
