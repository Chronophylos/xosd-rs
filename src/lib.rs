//! xosd-rs is a rust library with bindings to the xosd C/C++ library.
//!
//! The API is very similar to the original. The main difference is that all
//! functions are implemented in [`Xosd`].
//!
//! All functions return [`Result`] since it builds on unsafe code.
//!
//! [`Drop`] is implemented for [`Xosd`].
//!
//! # Example
//! Taken from the xosd man page:
//! ```rust
//! use xosd_rs::{Xosd, Command};
//!
//! let mut osd = Xosd::new(1)?;
//!
//! osd.set_font("fixed")?;
//! osd.set_color("LawnGreen")?;
//! osd.set_timeout(3)?;
//! osd.set_shadow_offset(1)?;
//!
//! osd.display(0, Command::string("Example XOSD output")?)?;
//!
//! osd.wait_until_no_display()?;
//!
//! # Ok::<(), xosd_rs::Error>(())
//! ```
//!
//! More examples can be found [here](https://github.com/Chronophylos/xosd-rs/tree/main/examples).
#![doc(html_root_url = "https://docs.rs/xosd-rs/0.2.0")]

use std::{
    borrow::Cow,
    convert::TryInto,
    ffi::{CStr, CString},
    fmt,
    os::raw::c_uint,
};

use thiserror::Error;
use xosd_sys::*;

macro_rules! wrap_unsafe {
    ($fn:expr) => {
        if unsafe { $fn } != 0 {
            Err(Error::XosdError(error_str()?.into_owned()))
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

/// Various errors that can occur in this crate
#[derive(Debug, Error, PartialEq, Clone)]
pub enum Error {
    /// Used when a [`std::ffi::IntoStringError`] occurs
    #[error("Could not convert CString into String: {0}")]
    IntoStringError(
        #[source]
        #[from]
        std::ffi::IntoStringError,
    ),

    #[error("Pointer is null")]
    IsNullPtr,

    /// Used for any error raised by the XOSD library
    #[error("Error in xosd.h: {0}")]
    XosdError(String),

    #[error("Cannot create a xosd object with zero or less lines")]
    InvalidLineCount,

    #[error("Percentage must be between 1 and 100")]
    OutOfRangePercentage,

    /// Used when a [`std::ffi::NulError`] occurs
    #[error("Could not create CString from String")]
    CStringNullError(
        #[source]
        #[from]
        std::ffi::NulError,
    ),

    #[error("Could convert from int")]
    TryFromIntError(
        #[source]
        #[from]
        std::num::TryFromIntError,
    ),
}

/// A helpful type to reduce repeated code
pub type Result<T> = std::result::Result<T, Error>;

fn error_str<'a>() -> Result<Cow<'a, str>> {
    wrap_static_string!(xosd_error)
}

/// Get the default color
///
/// The XOSD library defines and uses a default color. This can be queries here.
/// The returned string represents a name from the X11
/// [`rgb.txt`](https://gitlab.freedesktop.org/xorg/app/rgb/raw/master/rgb.txt).
///
/// # Errors
///
/// If `osd_default_color` points to NULL [`Error::IsNullPtr`] is returned.
///
/// # Example
///
/// ```
/// # use xosd_rs::{Xosd, default_color};
/// let mut osd = Xosd::new(1)?;
///
/// assert_eq!(default_color()?, "green");
///
/// # Ok::<(), xosd_rs::Error>(())
/// ```
pub fn default_color<'a>() -> Result<Cow<'a, str>> {
    wrap_static_string!(osd_default_colour)
}

/// Get the default font
///
/// The XOSD library defines and uses a default font. This can be queries here.
/// The returned string represents a X11 font name description.
///
/// # Errors
///
/// If `osd_default_font` points to NULL [`Error::IsNullPtr`] is returned.
///
/// # Example
///
/// ```
/// # use xosd_rs::{Xosd, default_font};
/// let mut osd = Xosd::new(1)?;
///
/// assert_eq!(default_font()?, "-misc-fixed-medium-r-semicondensed--*-*-*-*-c-*-*-*");
///
/// # Ok::<(), xosd_rs::Error>(())
/// ```
pub fn default_font<'a>() -> Result<Cow<'a, str>> {
    wrap_static_string!(osd_default_font)
}

/// Various types that can be displayed with [`Xosd::display`]
///
/// You should not construct any of these variants manually. Instead use one of
/// the constructors below.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Command {
    /// Used to display a percentage
    Percentage(u16),

    /// Used to display text
    String(String),

    /// Used to display a slider
    Slider(u16),
}

impl Command {
    /// Construct the [`Command::Percentage`] variant
    ///
    /// # Errors
    ///
    /// If precentage is greater than 100 or less than 1 return [`Error::OutOfRangePercentage`]
    pub fn percentage(percentage: u16) -> Result<Self> {
        if percentage > 100 || percentage < 1 {
            Err(Error::OutOfRangePercentage)
        } else {
            Ok(Self::Percentage(percentage))
        }
    }

    /// Construct the [`Command::String`] variant
    ///
    /// # Errors
    ///
    /// This function never fails
    pub fn string<S>(string: S) -> Result<Self>
    where
        S: ToString,
    {
        Ok(Self::String(string.to_string()))
    }

    /// Construct the [`Command::Slider`] variant
    ///
    /// # Errors
    ///
    /// If precentage is greater than 100 or less than 1 return [`Error::OutOfRangePercentage`]
    pub fn slider(slider: u16) -> Result<Self> {
        if slider > 100 || slider < 1 {
            Err(Error::OutOfRangePercentage)
        } else {
            Ok(Self::Slider(slider))
        }
    }
}

/// Represents the 3 different vertical alignments
///
/// This enum is used in [`Xosd::set_vertical_align`]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum VerticalAlign {
    Top,
    Center,
    Bottom,
}

impl fmt::Display for VerticalAlign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Top => "top",
                Self::Center => "center",
                Self::Bottom => "bottom",
            }
        )
    }
}

#[doc(hidden)]
impl Into<xosd_pos> for VerticalAlign {
    fn into(self) -> xosd_pos {
        match self {
            Self::Top => xosd_pos_XOSD_top,
            Self::Center => xosd_pos_XOSD_middle,
            Self::Bottom => xosd_pos_XOSD_bottom,
        }
    }
}

/// Represents the 3 different horizontal alignments
///
/// This enum is used in [`Xosd::set_horizontal_align`]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum HorizontalAlign {
    Left,
    Center,
    Right,
}

impl fmt::Display for HorizontalAlign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Left => "left",
                Self::Center => "center",
                Self::Right => "right",
            }
        )
    }
}

#[doc(hidden)]
impl Into<xosd_align> for HorizontalAlign {
    fn into(self) -> xosd_pos {
        match self {
            Self::Left => xosd_align_XOSD_left,
            Self::Center => xosd_align_XOSD_center,
            Self::Right => xosd_align_XOSD_right,
        }
    }
}

#[derive(Debug, Clone, Hash)]
pub struct Xosd(*mut xosd);

/// Calls the destructor for the XOSD object.
///
/// # Panics
///
/// If `xsod_uninit` fails. Or if getting the error message fails after
/// destroying the XOSD object fails.
impl Drop for Xosd {
    fn drop(&mut self) {
        if unsafe { xosd_uninit(self.0) } != 0 {
            panic!(
                "Could not destruct xosd instance: {}",
                error_str().expect("the error message while panicing after `xosd_uninit` failed")
            )
        }
    }
}

impl Xosd {
    /// Create a new [`Xosd`] object.
    ///
    /// This creates a new xosd window that can be used to display textual or
    /// numerical data on a X11 display in a unmanaged, shaped window that
    /// appears to be transparent. It provides a similar effect to the on-screen
    /// display of many televisions and video recorders.
    ///
    /// `lines` is the maximum number of lines that the window can display.
    ///
    /// # Errors
    ///
    /// * If `lines` is less than 1 [`Error::InvalidLineCount`] is returned.
    /// * If `xosd_create` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::{Xosd, Command};
    /// let mut osd = Xosd::new(1)?;
    ///
    /// osd.display(0, Command::string("Example XOSD output")?)?;
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn new<'a>(lines: i32) -> Result<Self> {
        if lines == 0 {
            return Err(Error::InvalidLineCount);
        }

        let xosd = unsafe { xosd_create(lines.into()) };

        if !xosd.is_null() {
            Ok(Self(xosd))
        } else {
            Err(Error::XosdError(error_str()?.into_owned()))
        }
    }

    /// Change the length of the percentage bar or slider.
    ///
    /// This changes the percentage of the display used by a slider or percentage
    /// bar. Normally the XOSD choses a sensible length for the bar, but you may
    /// wish to change the default behavior if there are only a small number of
    /// possible values to be displayed.
    ///
    /// `percentage` is the percentage of the display to be used up by the slider
    /// or percentage bar, as an interger between 0 and 100. Passing [`None`]
    /// reverts to the default behaviour.
    ///
    /// # Errors
    ///
    /// * If `percentage` is greater than 100 [`Error::OutOfRangePercentage`] is
    /// returned.
    /// * If `xosd_set_bar_lengh` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::Xosd;
    /// let mut osd = Xosd::new(1)?;
    ///
    /// osd.set_bar_length(Some(10))?;  // Set lenght to 10%
    /// osd.set_bar_length(None)?;      // Automatically determine length
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn set_bar_length(&mut self, percentage: Option<u16>) -> Result<()> {
        if let Some(percentage) = percentage {
            if percentage > 100 {
                return Err(Error::OutOfRangePercentage);
            }
        }

        wrap_unsafe!(xosd_set_bar_length(
            self.0,
            percentage.map(|v| v.into()).unwrap_or(-1)
        ))
    }

    /// Display data to an XOSD window.
    ///
    /// This function displays a `Command` to the XOSD window.
    ///
    /// This function returns immediatly but the data is displayed until the
    /// timeout limit is reached. You can set the timeout with
    /// [`Xosd::set_timeout`]. Use [`Xosd::wait_until_no_display`] to block until
    /// the data is not displayed anymore. A window that is displaying data can
    /// be hidden by calling [`Xosd::hide`].
    ///
    /// # Returns
    ///
    /// * If `command` is [`Command::String`] the number of characters written is
    /// returned.
    /// * If `command` is [`Command::Percentage`] or [`Command::Slider`] the
    /// value of the bar is returned.
    ///
    /// # Errors
    ///
    /// * If `xosd_display` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::{Xosd, Command};
    /// let mut osd = Xosd::new(2)?;
    ///
    /// let message = "A message on your screen";
    /// assert_eq!(osd.display(0, Command::string(&message)?)?, message.len() as u16);
    ///
    /// assert_eq!(osd.display(1, Command::percentage(13)?)?, 13);
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
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
            Err(Error::XosdError(error_str()?.into_owned()))
        } else {
            Ok(res
                .try_into()
                .expect("res is positive and should fit into u16"))
        }
    }

    /// Returns wether the XOSD window is shown.
    ///
    /// Determines wether a XOSD window is currently beeing shown.
    ///
    /// Use [`Xosd::show`] and [`Xosd::hide`] to alter the visibility of the XOSD
    /// window.
    ///
    /// # Errors
    ///
    /// * If `xosd_is_onscreen` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::Xosd;
    /// let mut osd = Xosd::new(1)?;
    ///
    /// assert_eq!(osd.onscreen()?, false);
    ///
    /// osd.show()?;
    ///
    /// assert_eq!(osd.onscreen()?, true);
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn onscreen(&mut self) -> Result<bool> {
        match unsafe { xosd_is_onscreen(self.0) } {
            1 => Ok(true),
            0 => Ok(false),
            _ => Err(Error::XosdError(error_str()?.into_owned())),
        }
    }

    /// Wait until the XOSD window is not displaying anything.
    ///
    /// Block until the timeout limit is reached and nothing is displayed on the XOSD window.
    ///
    /// # Errors
    ///
    /// * If `xosd_wait_until_no_display` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::{Xosd, Command};
    /// let mut osd = Xosd::new(1)?;
    /// osd.set_timeout(1)?;
    ///
    /// osd.display(0, Command::string("Example XOSD output")?)?;
    ///
    /// if osd.onscreen()? {
    ///     osd.wait_until_no_display()?;
    /// }
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn wait_until_no_display(&mut self) -> Result<()> {
        wrap_unsafe!(xosd_wait_until_no_display(self.0))
    }

    /// Hide the XOSD window
    ///
    /// This unmaps the XOSD window. Use [`Xosd::show`] to remap it.
    ///
    /// If [`Xosd::display`] is used when the window is not visible, the window becomes visible again.
    ///
    /// # Errors
    ///
    /// * If `xosd_hide` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use xosd_rs::{Xosd, Command};
    /// let mut osd = Xosd::new(1)?;
    ///
    /// assert!(!osd.onscreen()?, "XOSD window initializes as hidden");
    ///
    /// osd.display(0, Command::string("Example XOSD output")?)?;
    ///
    /// assert!(osd.onscreen()?, "using Xosd::display shows the display");
    ///
    /// osd.hide()?;
    ///
    /// assert!(!osd.onscreen()?, "after using Xosd::hide the windows is not visible anymore");
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    // BUG: example fails
    pub fn hide(&mut self) -> Result<()> {
        wrap_unsafe!(xosd_hide(self.0))
    }

    /// Show the XOSD window
    ///
    /// Redisplay the data that has been previously displayed by [`Xosd::display`].
    ///
    /// # Errors
    ///
    /// * If `xosd_show` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use xosd_rs::{Xosd, Command};
    /// let mut osd = Xosd::new(1)?;
    ///
    /// assert!(!osd.onscreen()?, "XOSD window initializes as hidden");
    ///
    /// osd.display(0, Command::string("Example XOSD output")?)?;
    ///
    /// assert!(osd.onscreen()?, "using Xosd::display shows the display");
    ///
    /// osd.hide()?;
    ///
    /// assert!(!osd.onscreen()?, "after using Xosd::hide the windows is not visible anymore");
    ///
    /// osd.show()?;
    ///
    /// assert!(!osd.onscreen()?, "after using Xosd::show the windows is visible again");
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    // BUG: example fails
    pub fn show(&mut self) -> Result<()> {
        wrap_unsafe!(xosd_show(self.0))
    }

    /// Change the vertical alignment of the XOSD window
    ///
    /// # Errors
    ///
    /// * If `xosd_set_pos` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::{Xosd, VerticalAlign};
    /// let mut osd = Xosd::new(1)?;
    ///
    /// osd.set_vertical_align(VerticalAlign::Top)?;
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn set_vertical_align(&mut self, align: VerticalAlign) -> Result<()> {
        wrap_unsafe!(xosd_set_pos(self.0, align.into()))
    }

    /// Change the horizontal alignment of the XOSD window
    ///
    /// # Errors
    ///
    /// * If `xosd_set_align` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::{Xosd, HorizontalAlign};
    /// let mut osd = Xosd::new(1)?;
    ///
    /// osd.set_horizontal_align(HorizontalAlign::Right)?;
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn set_horizontal_align(&mut self, align: HorizontalAlign) -> Result<()> {
        wrap_unsafe!(xosd_set_align(self.0, align.into()))
    }

    /// Change the shadow offset of the XOSD window
    ///
    /// XOSD draws a shadow beneath the main XOSD window to increase readability.
    ///
    /// Change the size of the XOSD window by altering how many pixels the shadow
    /// is offset to the bottom-right. One to four pixels result in a good
    /// effect.
    ///
    /// # Errors
    ///
    /// * If `xosd_set_shadow_offset` fails the xosd error message is wrapped in
    /// a [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::Xosd;
    /// let mut osd = Xosd::new(1)?;
    ///
    /// osd.set_shadow_offset(2)?;
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn set_shadow_offset(&mut self, offset: i32) -> Result<()> {
        wrap_unsafe!(xosd_set_shadow_offset(self.0, offset))
    }

    /// Change the outline offset of the text
    ///
    /// XOSD draws a outline around the text on the XOSD window.
    ///
    /// Change the thickness of the outline in pixels.
    ///
    /// # Errors
    ///
    /// * If `xosd_set_outline_offset` fails the xosd error message is wrapped in
    /// a [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::Xosd;
    /// let mut osd = Xosd::new(1)?;
    ///
    /// osd.set_outline_offset(2)?;
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn set_outline_offset(&mut self, offset: i32) -> Result<()> {
        wrap_unsafe!(xosd_set_outline_offset(self.0, offset))
    }

    /// Set the shadow color of the XOSD window
    ///
    /// Change the color to one defined by X11 in
    /// [`rgb.txt`](https://gitlab.freedesktop.org/xorg/app/rgb/raw/master/rgb.txt).
    ///
    /// # Errors
    ///
    /// * If `xosd_set_shadow_colour` fails the xosd error message is wrapped in
    /// a [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::Xosd;
    /// let mut osd = Xosd::new(1)?;
    ///
    /// osd.set_shadow_color("White")?;
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn set_shadow_color<S>(&mut self, color: S) -> Result<()>
    where
        S: Into<Vec<u8>>,
    {
        wrap_unsafe!(xosd_set_shadow_colour(
            self.0,
            CString::new(color)?.as_ptr()
        ))
    }

    /// Set the outline color of the text
    ///
    /// Change the color to one defined by X11 in
    /// [`rgb.txt`](https://gitlab.freedesktop.org/xorg/app/rgb/raw/master/rgb.txt).
    ///
    /// # Errors
    ///
    /// * If `xosd_set_outline_colour` fails the xosd error message is wrapped in
    /// a [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::Xosd;
    /// let mut osd = Xosd::new(1)?;
    ///
    /// osd.set_outline_color("Grey")?;
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn set_outline_color<S>(&mut self, color: S) -> Result<()>
    where
        S: Into<Vec<u8>>,
    {
        wrap_unsafe!(xosd_set_shadow_colour(
            self.0,
            CString::new(color)?.as_ptr()
        ))
    }

    /// Change the horizontal offset of the XOSD window
    ///
    /// Changes the number of pixels the XOSD window is offset from left of the
    /// screen.
    ///
    /// # Errors
    ///
    /// * If `xosd_set_horizontal_offset` fails the xosd error message is wrapped in
    /// a [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::Xosd;
    /// let mut osd = Xosd::new(1)?;
    ///
    /// osd.set_horizontal_offset(20)?;
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn set_horizontal_offset(&mut self, offset: i32) -> Result<()> {
        wrap_unsafe!(xosd_set_horizontal_offset(self.0, offset))
    }

    /// Change the vertical offset of the XOSD window
    ///
    /// Changes the number of pixels the XOSD window is offset from the top or
    /// bottom of the screen. Set this to 48 to avoid desktop-panels like in
    /// GNOME or KDE.
    ///
    /// # Errors
    ///
    /// * If `xosd_set_vertical_offset` fails the xosd error message is wrapped in
    /// a [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::Xosd;
    /// let mut osd = Xosd::new(1)?;
    ///
    /// osd.set_vertical_offset(48)?;
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn set_vertical_offset(&mut self, offset: i32) -> Result<()> {
        wrap_unsafe!(xosd_set_vertical_offset(self.0, offset))
    }

    /// Change the time until the XOSD window is hidden.
    ///
    /// Changes the number of seconds to wait after displaying data to hide the
    /// XOSD window.
    ///
    /// # Errors
    ///
    /// * If `xosd_set_timeout` fails the xosd error message is wrapped in
    /// a [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::{Xosd, Command};
    /// let mut osd = Xosd::new(1)?;
    ///
    /// osd.set_timeout(3)?;
    ///
    /// osd.display(0, Command::string("Test")?)?;
    ///
    /// if osd.onscreen()? {
    ///     osd.wait_until_no_display()?;
    /// }
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn set_timeout(&mut self, timeout: u16) -> Result<()> {
        wrap_unsafe!(xosd_set_timeout(self.0, timeout.into()))
    }

    /// Change the text color
    ///
    /// Change the color to one defined by X11 in
    /// [`rgb.txt`](https://gitlab.freedesktop.org/xorg/app/rgb/raw/master/rgb.txt).
    ///
    /// # Errors
    ///
    /// * If `xosd_set_colour` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::Xosd;
    /// let mut osd = Xosd::new(1)?;
    ///
    /// osd.set_color("LimeGreen")?;
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn set_color<S>(&mut self, color: S) -> Result<()>
    where
        S: Into<Vec<u8>>,
    {
        wrap_unsafe!(xosd_set_colour(self.0, CString::new(color)?.as_ptr()))
    }

    /// Change the text font
    ///
    /// Changes the font used to render text on the XOSD window. A X11 font
    /// description name must be passed.
    ///
    ///
    /// # Errors
    ///
    /// * If `xosd_set_font` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::Xosd;
    /// let mut osd = Xosd::new(1)?;
    ///
    /// osd.set_font("fixed")?;
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn set_font<S>(&mut self, font: S) -> Result<()>
    where
        S: Into<Vec<u8>>,
    {
        wrap_unsafe!(xosd_set_font(self.0, CString::new(font)?.as_ptr()))
    }

    /// Get the current text color
    ///
    /// Returns a RGB8 tuple with (red, green, blue). XOSD originally returns
    /// RGB16 but since X11 RGB colors are defined as RGB8, it gets converted to
    /// RGB8.
    ///
    /// # Errors
    ///
    /// If `xosd_get_colour` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned. If the color conversion from u16 to u8
    /// fails [`Error::TryFromIntError`] gets returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::Xosd;
    /// let mut osd = Xosd::new(1)?;
    ///
    /// assert_eq!(osd.color()?, (0, 255, 0));
    ///
    /// osd.set_color("LimeGreen")?;
    ///
    /// assert_eq!(osd.color()?, (50, 205, 50));
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn color(&mut self) -> Result<(u8, u8, u8)> {
        let mut red = 0;
        let mut green = 0;
        let mut blue = 0;

        wrap_unsafe!(xosd_get_colour(self.0, &mut red, &mut green, &mut blue))?;

        Ok((
            (red / 256).try_into()?,
            (green / 256).try_into()?,
            (blue / 256).try_into()?,
        ))
    }

    /// Scroll the display
    ///
    /// Scrolls the display by a number of lines up
    ///
    /// # Errors
    ///
    /// * If `xosd_get_number_lines` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::{Xosd, Command};
    /// let mut osd = Xosd::new(2)?;
    ///
    /// osd.display(0, Command::string("Hello,")?)?;
    /// osd.display(1, Command::string("World!")?)?;
    ///
    /// // The display shows:
    /// // Hello,
    /// // World!
    ///
    /// osd.scroll(1)?;
    ///
    /// // The display shows:
    /// // World!
    /// //
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn scroll(&mut self, lines: i32) -> Result<()> {
        wrap_unsafe!(xosd_scroll(self.0, lines))
    }

    /// Get the maximum number of lines that can be displayed on the XOSD window.
    ///
    /// # Errors
    ///
    /// * If `xosd_get_number_lines` fails the xosd error message is wrapped in a
    /// [`Error::XosdError`] and returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use xosd_rs::Xosd;
    /// assert_eq!(Xosd::new(1)?.max_lines()?, 1);
    /// assert_eq!(Xosd::new(123)?.max_lines()?, 123);
    ///
    /// # Ok::<(), xosd_rs::Error>(())
    /// ```
    pub fn max_lines(&mut self) -> Result<i32> {
        let res = unsafe { xosd_get_number_lines(self.0) };

        if res < 0 {
            Err(Error::XosdError(error_str()?.into_owned()))
        } else {
            Ok(res.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "success depends on test order"]
    fn test_error() {
        assert_eq!(error_str().unwrap(), Cow::from(""))
    }

    #[test]
    fn test_new() {
        drop(Xosd::new(12).unwrap())
    }

    #[test]
    fn test_new_zero_line() {
        assert_eq!(Xosd::new(0).err(), Some(Error::InvalidLineCount))
    }
}
