use std::io::Error;

use std::io;
use std::net::UdpSocket;

// need to update for multiple peer?
// currently search only single peer.
pub fn search_peers() -> std::io::Result<UdpSocket> {
    use std::net::*;
    let multicast = Ipv4Addr::new(0b11101111, 255, 2, 134);
    let port = 21340;
    assert!(multicast.is_multicast());
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port)).expect("failed to bind");
    socket
        .join_multicast_v4(&multicast, &Ipv4Addr::UNSPECIFIED)
        .expect("failed to join multicast");
    socket
        .set_multicast_loop_v4(false)
        .expect("failed to disable multicast loop");
    let timeout = std::time::Duration::from_millis(100);
    socket
        .set_write_timeout(Some(timeout))
        .expect("failed to set write timeout");
    socket
        .set_read_timeout(Some(timeout))
        .expect("failed to set read timeout");
    const MSG: &[u8; 9] = b"cat-choco";
    let mut counter = 0..1000;
    let peer = loop {
        if counter.next().is_none() {
            break Err(std::io::Error::other("could not find peer"));
        }
        let sent = socket
            .send_to(MSG, (multicast, port))
            .expect("failed to send");
        assert!(sent == MSG.len());
        let mut buf = [0u8; MSG.len()];
        let Ok((recv, addr)) = socket.recv_from(&mut buf) else {
            continue;
        };
        if recv == MSG.len() && buf == *MSG {
            break Ok(addr);
        }
    }?;
    socket.connect(peer).expect("failed to connect");
    socket
        .leave_multicast_v4(&multicast, &Ipv4Addr::UNSPECIFIED)
        .expect("failed to leave multicast");

    Ok(socket)
}
