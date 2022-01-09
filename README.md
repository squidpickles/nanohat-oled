This is a Rust port of the manufacturer code for the NanoHat OLED, based on the [SSD1306](https://www.solomon-systech.com/product/ssd1306/) OLED display from Solomon Systech.

It enables basic access to the screen functions, including a facility for displaying text and images.

## Example
```rust
use nanohat_oled::{Oled, OledResult};

fn main() -> OledResult {
    let mut oled = Oled::from_path("/dev/i2c-0")?;
    oled.init()?;
    oled.put_string("Hello, world!")?;
    Ok(())
}
```
