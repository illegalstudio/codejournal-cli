use std::fmt::Arguments;
use std::io::{self, ErrorKind, Write};

// All application stdout writes use these macros instead of the panicking standard macros.
macro_rules! print {
    ($($args:tt)*) => { $crate::stdout::write(format_args!($($args)*)) };
}
macro_rules! println {
    () => { $crate::stdout::write(format_args!("\n")) };
    ($($args:tt)*) => { $crate::stdout::write(format_args!("{}\n", format_args!($($args)*))) };
}
pub(crate) use print;
pub(crate) use println;

pub fn write(args: Arguments<'_>) {
    let mut stdout = io::stdout().lock();
    if let Err(error) = stdout.write_fmt(args).and_then(|()| stdout.flush()) {
        if error.kind() == ErrorKind::BrokenPipe {
            // A consumer such as head has enough output; ending here is normal CLI behavior.
            std::process::exit(0);
        }
        let _ = writeln!(io::stderr().lock(), "Error: stdout: {error}");
        std::process::exit(1);
    }
}
