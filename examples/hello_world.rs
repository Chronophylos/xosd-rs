use xosd_rs::{Command, HorizontalAlign, Result, VerticalAlign, Xosd};

fn main() -> Result<()> {
    let mut osd = Xosd::new(2)?;

    osd.set_timeout(3)?;
    osd.set_vertical_align(VerticalAlign::Center)?;
    osd.set_horizontal_align(HorizontalAlign::Center)?;

    osd.display(0, Command::string("Hello,")?)?;
    osd.display(1, Command::string("World!")?)?;

    if osd.onscreen()? {
        osd.wait_until_no_display()?;
    }

    Ok(())
}
