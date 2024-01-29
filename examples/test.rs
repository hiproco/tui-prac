use tui_prac;

fn main() -> std::io::Result<()> {
    let socket = tui_prac::network::search_peers()?;
    eprintln!("local:{:?}", socket.local_addr()?);
    eprintln!("peer:{:?}", socket.peer_addr()?);
    socket.send(b"hello")?;
    let mut buf = [0u8; 100];
    let _n = socket.recv(&mut buf)?;
    eprintln!("{}", String::from_utf8_lossy(&buf));
    Ok(())
}
