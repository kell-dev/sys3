/// The global writer implementation.
pub static GLOBAL_WRITER: OnceCell<LockedWriter> = OnceCell::uninit();

/// Initialise a global writer using the framebuffer set up by the bootloader.
pub fn init_writer(
   buffer: &'static mut [u8],
   info: FrameBufferInfo,
   with_framebuffer: bool,
   with_serial: bool,
) {
   let writer = GLOBAL_WRITER.get_or_init(move || {
      LockedWriter::new(buffer, info, with_framebuffer, with_serial)
   });

   log::set_logger(writer).expect("logger already exists");
   log::set_max_level(LevelFilter::Trace);
   log::info!("Global writer/logger successfully initialised: {:?}", info);
}

pub struct LockedWriter {
   pub writer: Option<Spinlock<TerminalWriter>>,
   pub serial: Option<Spinlock<SerialPort>>,
}

impl LockedWriter {
   pub fn new(
      buffer: &'static mut [u8],
      info: FrameBufferInfo,
      writer_log_status: bool,
      serial_log_status: bool,
   ) -> Self {
      let port = unsafe {
         let mut serial = SerialPort::new(0x3F8);
         serial.init();
         serial
      };

      let writer = match writer_log_status {
         true => Some(Spinlock::new(TerminalWriter::new(buffer, info))),
         false => None,
      };

      let serial = match serial_log_status {
         true => Some(Spinlock::new(port)),
         false => None,
      };

      return LockedWriter {
         writer,
         serial,
      };
   }

   /// Force-unlocks the logger to prevent a deadlock.
   ///
   /// ## Safety
   /// This method is not memory safe and should be only used when absolutely necessary.
   pub unsafe fn force_unlock(&self) {
      if let Some(framebuffer) = &self.writer {
         unsafe { framebuffer.force_unlock() };
      }

      if let Some(serial) = &self.serial {
         unsafe { serial.force_unlock() };
      }
   }
}

impl log::Log for LockedWriter {
   fn enabled(&self, _metadata: &log::Metadata) -> bool {
      true
   }

   fn log(&self, record: &log::Record) {
      if let Some(writer) = &self.writer {
         let mut writer = writer.lock();
         writeln!(writer, "{:5}: {}", record.level(), record.args()).unwrap();
      }

      if let Some(serial) = &self.serial {
         let mut serial = serial.lock();
         writeln!(serial, "{:5}: {}", record.level(), record.args()).unwrap();
      }
   }

   fn flush(&self) {}
}

// MACROS //

#[macro_export]
macro_rules! print {
   ($($args:tt)+) => ({
      use core::fmt::Write;

      if let Some(writer) = &$crate::terminal::GLOBAL_WRITER.get().unwrap().writer {
         let mut writer = writer.lock();
         let _ = write!(writer, $($args)+).unwrap();
      }

      if let Some(serial) = &$crate::terminal::GLOBAL_WRITER.get().unwrap().serial {
         let mut serial = serial.lock();
         let _ = write!(serial, $($args)+).unwrap();
      }
   });
}

#[macro_export]
macro_rules! println {
   () => ({
      print!("\n");
   });

   ($fmt:expr) => ({
      print!(concat!($fmt, "\r\n"))
   });

   ($fmt:expr, $($args:tt)+) => ({
      print!(concat!($fmt, "\r\n"), $($args)+)
   });
}

#[macro_export]
pub macro clear_screen {
   () => {
      if let Some(writer) = &$crate::terminal::GLOBAL_WRITER.get().unwrap().writer {
         let mut writer = writer.lock();
         writer.clear();
      }
   }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
   use core::fmt::Write;

   if let Some(writer) = &GLOBAL_WRITER.get().unwrap().writer {
      let mut writer = writer.lock();
      writer.write_fmt(args).unwrap();
   }

   if let Some(serial) = &GLOBAL_WRITER.get().unwrap().serial {
      let mut serial = serial.lock();
      serial.write_fmt(args).unwrap();
   }
}

// MODULES //

/// Font-related constants.
pub mod font;

/// A framebuffer-based writer implementation that piggy-backs off the buffer
/// set up by the bootloader.
pub mod framebuffer;

// IMPORTS //

use {
   crate::uart::SerialPort,
   self::framebuffer::TerminalWriter,
   conquer_once::spin::OnceCell,
   core::fmt::{self, Write},
   log::LevelFilter,
   spinning_top::Spinlock,
   springboard_api::info::{FrameBufferInfo, PixelFormat},
};
