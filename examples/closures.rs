fn main() {
    let mut a = [0u8; 100];
    let mut l = 0;
    let push = |v: u8| -> Result<(), u8> {
        let e = a.get_mut(l).ok_or(v)?;
        *e = v;
        l += 1;
        Ok(())
    };
    match (0..100).try_for_each(push) {
        Ok(()) => {}
        Err(e) => eprintln!("failed to push: {}", e),
    }
    eprintln!("{:?}", a);
    
}
