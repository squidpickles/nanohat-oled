use nanohat_oled::Oled;
use std::io::Result;

fn run() -> Result<()> {
    let mut oled = Oled::from_path("/dev/i2c-0")?;
    oled.init()?;
    oled.put_string("Hello, world!")?;
    Ok(())
}

fn main() {
    run().unwrap();
}
