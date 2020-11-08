macro_rules! wrap_unsafe {
    ($fn:expr) => {
        if unsafe { $fn } != 0 {
            Err(Error::XosdError(get_error_str()?.into_owned()))
        } else {
            Ok(())
        }
    };
}

macro_rules! wrap_static_string {
    ($s:expr) => {
        match unsafe { $s.is_null() } {
            true => Err(Error::IsNullPtr),
            false => Ok(unsafe { CStr::from_ptr($s) }.to_string_lossy()),
        }
    };
}

use std::{
    borrow::Cow,
    convert::TryInto,
    ffi::{CStr, CString},
    os::raw::c_uint,
};

use thiserror::Error;
use xosd_sys::*;

#[derive(Debug, Error, PartialEq)]
pub enum Error {
    #[error("Could not convert CString into String: {0}")]
    IntoStringError(
        #[source]
        #[from]
        std::ffi::IntoStringError,
    ),

    #[error("Pointer is null")]
    IsNullPtr,

    #[error("Error in xosd.h: {0}")]
    XosdError(String),

    #[error("Cannot create a Xosd instance with zero lines")]
    ZeroLineNumber,

    #[error("Value is out of range (between 0 and 100)")]
    OutOfRange,

    #[error("Could not create CString from String")]
    CStringNullError(
        #[source]
        #[from]
        std::ffi::NulError,
    ),
}

pub type Result<T> = std::result::Result<T, Error>;

fn get_error_str<'a>() -> Result<Cow<'a, str>> {
    wrap_static_string!(xosd_error)
}

pub fn default_color<'a>() -> Result<Cow<'a, str>> {
    wrap_static_string!(osd_default_colour)
}

pub fn default_font<'a>() -> Result<Cow<'a, str>> {
    wrap_static_string!(osd_default_font)
}

pub enum Command {
    Percentage(u16),
    String(String),
    Printf(String),
    Slider(u16),
}

impl Command {
    /// Construct the percentage variant.
    /// `Error::OutOfRange` is returned if precentage is greater than 100;
    pub fn percentage(percentage: u16) -> Result<Self> {
        if percentage > 100 {
            Err(Error::OutOfRange)
        } else {
            Ok(Self::Percentage(percentage))
        }
    }

    /// Construct the string variant
    pub fn string<S>(string: S) -> Self
    where
        S: ToString,
    {
        Self::String(string.to_string())
    }

    /// Construct the printf variant
    pub fn printf<S>(string: S) -> Self
    where
        S: ToString,
    {
        Self::Printf(string.to_string())
    }

    /// Construct the slider variant.
    /// `Error::OutOfRange` is returned if slider is greater than 100;
    pub fn slider(slider: u16) -> Result<Self> {
        if slider > 100 {
            Err(Error::OutOfRange)
        } else {
            Ok(Self::Slider(slider))
        }
    }
}

pub enum VerticalAlign {
    Top,
    Center,
    Bottom,
}

impl Into<xosd_pos> for VerticalAlign {
    fn into(self) -> xosd_pos {
        match self {
            Self::Top => xosd_pos_XOSD_top,
            Self::Center => xosd_pos_XOSD_middle,
            Self::Bottom => xosd_pos_XOSD_bottom,
        }
    }
}

pub enum HorizontalAlign {
    Left,
    Center,
    Right,
}

impl Into<xosd_align> for HorizontalAlign {
    fn into(self) -> xosd_pos {
        match self {
            Self::Left => xosd_align_XOSD_left,
            Self::Center => xosd_align_XOSD_center,
            Self::Right => xosd_align_XOSD_right,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Xosd(*mut xosd);

impl Drop for Xosd {
    fn drop(&mut self) {
        if unsafe { xosd_uninit(self.0) } != 0 {
            panic!(
                "Could not destruct xosd instance: {}",
                get_error_str().unwrap()
            )
        }
    }
}

impl Xosd {
    /// Create a new `Xosd` instance.
    pub fn new<'a>(lines: i32) -> Result<Self> {
        if lines == 0 {
            return Err(Error::ZeroLineNumber);
        }

        let xosd = unsafe { xosd_create(lines.into()) };

        if !xosd.is_null() {
            Ok(Self(xosd))
        } else {
            Err(Error::XosdError(get_error_str()?.into_owned()))
        }
    }

    /// Set length of percentage and slider bar.
    /// Pass `None` for "old behaviour".
    pub fn set_bar_length(&mut self, length: Option<u16>) -> Result<()> {
        wrap_unsafe!(xosd_set_bar_length(
            self.0,
            length.map(|v| v.into()).unwrap_or(-1)
        ))
    }

    pub fn display(&mut self, line: i32, command: Command) -> Result<u16> {
        let res = match command {
            Command::Percentage(percentage) => unsafe {
                xosd_display(
                    self.0,
                    line.into(),
                    xosd_command_XOSD_percentage,
                    percentage as c_uint,
                )
            },
            Command::String(string) => unsafe {
                xosd_display(
                    self.0,
                    line.into(),
                    xosd_command_XOSD_string,
                    CString::new(string)?,
                )
            },
            Command::Printf(string) => unsafe {
                xosd_display(
                    self.0,
                    line.into(),
                    xosd_command_XOSD_printf,
                    CString::new(string)?,
                )
            },
            Command::Slider(slider) => unsafe {
                xosd_display(
                    self.0,
                    line.into(),
                    xosd_command_XOSD_slider,
                    slider as c_uint,
                )
            },
        };

        if res < 0 {
            Err(Error::XosdError(get_error_str()?.into_owned()))
        } else {
            Ok(res
                .try_into()
                .expect("res is positive and should fit into u16"))
        }
    }

    /// Returns weather the display is shown
    pub fn is_onscreen(&mut self) -> Result<bool> {
        let res = unsafe { xosd_is_onscreen(self.0) };
        if res == 1 {
            Ok(true)
        } else if res == 0 {
            Ok(false)
        } else {
            Err(Error::XosdError(get_error_str()?.into_owned()))
        }
    }

    /// Wait until nothing is displayed
    pub fn wait_until_no_display(&mut self) -> Result<()> {
        wrap_unsafe!(xosd_wait_until_no_display(self.0))
    }

    /// Hide the display
    pub fn hide(&mut self) -> Result<()> {
        wrap_unsafe!(xosd_hide(self.0))
    }

    /// Show the display
    pub fn show(&mut self) -> Result<()> {
        wrap_unsafe!(xosd_show(self.0))
    }

    /// Set the vertical alignment of the display
    pub fn set_vertical_align(&mut self, align: VerticalAlign) -> Result<()> {
        wrap_unsafe!(xosd_set_pos(self.0, align.into()))
    }

    /// Set the horizontal alignment of the display
    pub fn set_horizontal_align(&mut self, align: HorizontalAlign) -> Result<()> {
        wrap_unsafe!(xosd_set_align(self.0, align.into()))
    }

    /// Set the offset of the text shadow
    pub fn set_shadow_offset(&mut self, offset: i32) -> Result<()> {
        wrap_unsafe!(xosd_set_shadow_offset(self.0, offset))
    }

    /// Set the offset of the text outline
    pub fn set_outline_offset(&mut self, offset: i32) -> Result<()> {
        wrap_unsafe!(xosd_set_outline_offset(self.0, offset))
    }

    /// Set the shadow color
    /// See X11 Color Names
    pub fn set_shadow_color<S>(&mut self, color: S) -> Result<()>
    where
        S: Into<Vec<u8>>,
    {
        wrap_unsafe!(xosd_set_shadow_colour(
            self.0,
            CString::new(color)?.as_ptr()
        ))
    }

    /// Set the outline color
    /// See X11 Color Names
    pub fn set_outline_color<S>(&mut self, color: S) -> Result<()>
    where
        S: Into<Vec<u8>>,
    {
        wrap_unsafe!(xosd_set_shadow_colour(
            self.0,
            CString::new(color)?.as_ptr()
        ))
    }

    pub fn set_horizontal_offset(&mut self, offset: i32) -> Result<()> {
        wrap_unsafe!(xosd_set_horizontal_offset(self.0, offset))
    }

    pub fn set_vertical_offset(&mut self, offset: i32) -> Result<()> {
        wrap_unsafe!(xosd_set_vertical_offset(self.0, offset))
    }

    pub fn set_timeout(&mut self, timeout: u16) -> Result<()> {
        wrap_unsafe!(xosd_set_timeout(self.0, timeout.into()))
    }

    pub fn set_color<S>(&mut self, color: S) -> Result<()>
    where
        S: Into<Vec<u8>>,
    {
        wrap_unsafe!(xosd_set_colour(self.0, CString::new(color)?.as_ptr()))
    }

    pub fn set_font<S>(&mut self, font: S) -> Result<()>
    where
        S: Into<Vec<u8>>,
    {
        wrap_unsafe!(xosd_set_font(self.0, CString::new(font)?.as_ptr()))
    }

    pub fn get_color(&mut self) -> Result<(i32, i32, i32)> {
        let mut red = 0;
        let mut green = 0;
        let mut blue = 0;

        wrap_unsafe!(xosd_get_colour(self.0, &mut red, &mut green, &mut blue))?;

        Ok((red.into(), green.into(), blue.into()))
    }

    pub fn scroll(&mut self, lines: i32) -> Result<()> {
        wrap_unsafe!(xosd_scroll(self.0, lines))
    }

    pub fn max_lines(&mut self) -> Result<i32> {
        let res = unsafe { xosd_get_number_lines(self.0) };

        if res != 0 {
            Err(Error::XosdError(get_error_str()?.into_owned()))
        } else {
            Ok(res.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // success depends on test order
    //#[test]
    //fn test_error() {
    //    assert_eq!(get_error_str().unwrap(), Cow::from(""))
    //}

    #[test]
    fn test_new() {
        drop(Xosd::new(12).unwrap())
    }

    #[test]
    fn test_new_zero_line() {
        assert_eq!(Xosd::new(0).err(), Some(Error::ZeroLineNumber))
    }
}
