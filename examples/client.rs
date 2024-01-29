use std::net::*;
fn main() {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 7890)).expect("failed to bind");
    let duration = std::time::Duration::from_millis(100);
    socket
        .join_multicast_v4(&Ipv4Addr::from([239, 255, 1, 234]), &Ipv4Addr::UNSPECIFIED)
        .expect("failed to join");
    socket
        .set_write_timeout(Some(duration))
        .expect("failed to set write timeout");
    socket
        .set_read_timeout(Some(duration))
        .expect("failed to set read timeout");
    let mut port = 0;
    for _ in 0..100 {
        let buf = &mut [0u8; 100];
        let Ok(recv) = socket.recv_from(buf) else {
            continue;
        };
        port = recv.1.port();
    }
    let port = port;
    let buf = &mut [0u8; 100];
    for _ in 0..100 {
        socket
            .send_to(
                b"test from client",
                (Ipv4Addr::from([239, 255, 1, 234]), port),
            )
            .expect("failed to send");
        let Ok(recv @ (_msg, _addr)) = socket.recv_from(buf) else {
            continue;
        };
        eprintln!("client:{:?}", recv);
    }
    eprintln!("client:{:?}", buf);
}
