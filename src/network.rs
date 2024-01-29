use std::{borrow::Cow, fmt::Display, io::Error, net::UdpSocket};

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
    peers
        .first()
        .ok_or(Error::other("no peer found"))
        .and_then(|p| socket.connect(p))?;
    socket
        .leave_multicast_v4(&multicast, &Ipv4Addr::UNSPECIFIED)
        .expect("failed to leave multicast");

    Ok(socket)
}
