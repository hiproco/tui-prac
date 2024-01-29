use std::net::*;
fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    eprintln!("{:?}", socket);
    // 239.0.0.0 - 239.255.255.255 (239.0.0.0\8)
    let multiaddr = &Ipv4Addr::new(239, 255, 1, 234);
    // ??? ?? perhaps a bind for the multicast ????
    let interface = &Ipv4Addr::UNSPECIFIED;

    socket
        .join_multicast_v4(multiaddr, interface)
        .expect("failed to join");

    let timeout = std::time::Duration::from_millis(100);
    socket
        .set_read_timeout(Some(timeout))
        .expect("failed to set read timeout");
    socket
        .set_write_timeout(Some(timeout))
        .expect("failed to set write timeout");
    let buf = &mut [0u8; 10];
    for _ in 0..100 {
        socket
            .send_to(b"from server", (*multiaddr, 7890))
            .expect("failed to send");
        if let Ok(recv @ (_read, addr)) = socket.recv_from(buf) {
            eprintln!("server:{:?}", recv);
            socket
                .send_to(b"hello from server side", addr)
                .expect("failed to send");
        } else {
            continue;
        }
        eprintln!("server:{:?}", buf);
    }

    socket
        .leave_multicast_v4(multiaddr, interface)
        .expect("failed to leave");

    // run_server();
    Ok(())
}

fn run_server() {}

// find peer using multicast ??
// search other ways too
// the parameter is the the last bits varaible in local multicast.
// use 123 as default
// port is port to use, default to 7890
fn find_peer(with: Option<u8>, port: Option<u16>) {
    let multicast = std::net::Ipv4Addr::new(224, 0, 0, with.unwrap_or(123));
    let port = port.unwrap_or(7890);
    let socketadd = std::net::SocketAddr::new(multicast.into(), port);
    let udp = std::net::UdpSocket::bind(socketadd).expect("failed to bind");
    let interface = &std::net::Ipv4Addr::UNSPECIFIED;
    udp.join_multicast_v4(&multicast, interface)
        .expect("failed to join");
    let timeout = std::time::Duration::from_millis(100);
    udp.set_read_timeout(Some(timeout))
        .expect("failed to set read timeout");
    udp.set_write_timeout(Some(timeout))
        .expect("failed to set write timeout");

    let buf = &mut [0u8; 128];
    for _ in 0..10000 {
        udp.send_to(b"test", socketadd).expect("failed to send");
        match udp.recv_from(buf) {
            Ok((_data, _peer)) => {}
            Err(e) => {
                dbg!(e);
            }
        }
    }

    udp.leave_multicast_v4(&multicast, interface)
        .expect("failed to leave");
}
