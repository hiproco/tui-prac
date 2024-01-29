use std::{borrow::Cow, fmt::Display, io::Error, net::UdpSocket};

trait CaptureError: Sized {
    fn pipe<A, E, F: FnOnce(&mut Self) -> Result<A, E>>(mut self, f: F) -> Result<Self, E> {
        f(&mut self)?;
        Ok(self)
    }
}
impl<T: Sized> CaptureError for T {}

// need to update for multiple peer?
// currently search only single peer.
pub fn search_peers() -> std::io::Result<UdpSocket> {
    use std::net::*;
    let multicast = Ipv4Addr::new(0b11101111, 255, 2, 134);
    let port = 21340;
    assert!(multicast.is_multicast());
    let with = |with: &str| {
        let with = with.to_string();
        move |e: Error| Error::new(e.kind(), with)
    };
    let timeout = Some(std::time::Duration::from_millis(100));

    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))
        // .into_iter()
        .expect("failed to bind")
        .pipe(|s| s.join_multicast_v4(&multicast, &Ipv4Addr::UNSPECIFIED))
        .expect("failed to join multicast")
        .pipe(|s| s.set_multicast_loop_v4(false))
        .expect("failed to disable multicast loop")
        .pipe(|s| s.set_write_timeout(timeout))
        .expect("failed to set write timeout")
        .pipe(|s| s.set_read_timeout(timeout))
        .expect("failed to set read timeout");
    const MSG: &[u8; 9] = b"cat-choco";
    let mut counter = (0..1000);
    let mut peers = vec![];
    counter.try_for_each(|_| -> std::io::Result<_> {
        let sent = socket.send_to(MSG, (multicast, port))?;
        assert!(sent == MSG.len());
        let mut buf = MSG.clone();
        buf.fill(0);
        peers.extend(
            socket
                .recv_from(&mut buf)
                .ok()
                .filter(|(recv, _)| *recv == MSG.len() && buf == *MSG)
                .map(|(_, addr)| addr),
        );
        Ok(())
    })?;
    if peers.is_empty() {
        return Err(Error::other("no peer found"));
    }
    socket.connect(peers[0]).expect("failed to connect");
    socket
        .leave_multicast_v4(&multicast, &Ipv4Addr::UNSPECIFIED)
        .expect("failed to leave multicast");

    Ok(socket)
}
