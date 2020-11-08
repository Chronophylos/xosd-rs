use xosd_rs::{Command, HorizontalAlign, Result, VerticalAlign, Xosd};

fn main() -> Result<()> {
    let mut xosd = Xosd::new(2)?;

    xosd.set_timeout(3)?;
    xosd.set_vertical_align(VerticalAlign::Center)?;
    xosd.set_horizontal_align(HorizontalAlign::Center)?;

    xosd.display(0, Command::string("Hello,"))?;
    xosd.display(1, Command::string("World!"))?;

    if xosd.is_onscreen()? {
        xosd.wait_until_no_display()?;
    }

    Ok(())
}
