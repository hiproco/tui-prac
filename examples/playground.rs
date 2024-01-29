use std::os::unix::net;

trait Socket: Sized {
    type Address;
    fn bind() -> std::io::Result<Self>;
}
trait ConnectionSocket: Socket {}
trait ListenerSocket: Socket {}

fn main() -> std::io::Result<()> {
    let _streams: [net::UnixStream; 2] = net::UnixStream::pair()?.try_into().expect("not pair");

    struct Guard {}
    impl Drop for Guard {
        fn drop(&mut self) {
            todo!()
        }
    }

    Ok(())
}

trait Serializer {}
trait Serialize {
    type BYTES: AsRef<[u8]>;
    fn to_bytes(&self) -> Self::BYTES;
}

impl Serialize for u32 {
    type BYTES = [u8; 4];
    fn to_bytes(&self) -> Self::BYTES {
        self.to_be_bytes()
    }
}
impl Serialize for i32 {
    type BYTES = [u8; 4];
    fn to_bytes(&self) -> Self::BYTES {
        self.to_be_bytes()
    }
}

// netstat
// route -n
