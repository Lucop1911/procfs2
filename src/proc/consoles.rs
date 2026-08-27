use std::path::Path;

use bitflags::bitflags;

use crate::error::{Error, Result};
use crate::util::parse;

bitflags! {
    /// Console capabilities and driver-state flags from `/proc/consoles`.
    ///
    /// The kernel prints three capability letters (`R` read, `W` write,
    /// `U` unblank) followed by a seven-letter driver-state field
    /// (`E C B N p b a`). Each letter is present only when the
    /// corresponding flag is set, so each maps one-to-one onto a bit.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct ConsoleFlags: u16 {
        /// Console can be read.
        const READ = 1 << 0;
        /// Console can be written.
        const WRITE = 1 << 1;
        /// Console supports unblanking.
        const UNBLANK = 1 << 2;
        /// `CON_ENABLED`: console accepts printed output.
        const ENABLED = 1 << 3;
        /// `CON_CONSDEV`: preferred console for `/dev/console`.
        const CONSDEV = 1 << 4;
        /// `CON_BOOT`: console used during boot.
        const BOOT = 1 << 5;
        /// `CON_NBCON`: non-blocking console.
        const NBCON = 1 << 6;
        /// `CON_PRINTBUFFER`: console reads the printk buffer.
        const PRINTBUFFER = 1 << 7;
        /// `CON_BRL`: braille console.
        const BRAILLE = 1 << 8;
        /// `CON_ANYTIME`: console usable at any time.
        const ANYTIME = 1 << 9;
    }
}

/// A single console device from `/proc/consoles`.
///
/// The kernel writes the console name and its index back-to-back with no
/// separator (e.g. the VT console appears as `tty0`), so both are kept in
/// [`Console::name`] as a single string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Console {
    /// Console name, as printed by the kernel (e.g. `tty0`).
    pub name: Box<str>,
    /// Capabilities and driver-state flags (see [`ConsoleFlags`]).
    pub flags: ConsoleFlags,
    /// Device `major`, present only when the console has a device.
    pub major: Option<u32>,
    /// Device `minor`, present only when the console has a device.
    pub minor: Option<u32>,
}

/// Reads `/proc/consoles` and returns the registered console devices.
///
/// Each line is fixed-width: the name is padded to 20 columns, followed by
/// the three capability letters and a parenthesised seven-letter flag field.
/// Because the flag field can contain spaces, the line is parsed by locating
/// the parentheses rather than splitting on whitespace.
pub fn consoles() -> Result<Vec<Console>> {
    let path = Path::new("/proc/consoles");
    let bytes = parse::read_file(path)?;

    let mut out = Vec::new();

    for (line_num, line) in bytes
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .enumerate()
    {
        out.push(parse_line(line, path, line_num + 1)?);
    }

    Ok(out)
}

fn parse_line(line: &[u8], path: &Path, line_num: usize) -> Result<Console> {
    let line = parse::trim_end(line);

    let open = parse::memchr(b'(', line).ok_or_else(|| Error::Parse {
        path: path.to_path_buf(),
        line: line_num,
        msg: "missing '(' in consoles line",
    })?;
    let close = parse::memchr(b')', &line[open + 1..])
        .map(|i| open + 1 + i)
        .ok_or_else(|| Error::Parse {
            path: path.to_path_buf(),
            line: line_num,
            msg: "missing ')' in consoles line",
        })?;

    // Everything before '(' is `<name (20 wide)><3 capability letters> `.
    let head = parse::trim_end(&line[..open]);
    if head.len() < 3 {
        return Err(Error::Parse {
            path: path.to_path_buf(),
            line: line_num,
            msg: "console line too short",
        });
    }
    let caps = &head[head.len() - 3..];
    let name = parse::trim_end(&head[..head.len() - 3]);

    let name = std::str::from_utf8(name).map_err(|_| Error::Parse {
        path: path.to_path_buf(),
        line: line_num,
        msg: "invalid console name",
    })?;

    let mut flags = ConsoleFlags::empty();
    for (b, bit) in caps.iter().zip([
        ConsoleFlags::READ,
        ConsoleFlags::WRITE,
        ConsoleFlags::UNBLANK,
    ]) {
        if *b != b'-' {
            flags |= bit;
        }
    }
    for &b in &line[open + 1..close] {
        flags |= match b {
            b'E' => ConsoleFlags::ENABLED,
            b'C' => ConsoleFlags::CONSDEV,
            b'B' => ConsoleFlags::BOOT,
            b'N' => ConsoleFlags::NBCON,
            b'p' => ConsoleFlags::PRINTBUFFER,
            b'b' => ConsoleFlags::BRAILLE,
            b'a' => ConsoleFlags::ANYTIME,
            _ => ConsoleFlags::empty(),
        };
    }

    let dev = parse::trim(&line[close + 1..]);
    let (major, minor) = if dev.is_empty() {
        (None, None)
    } else {
        let (major, minor) = parse::split_at_byte(dev, b':');
        let major = parse::parse_dec_u32(major).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num,
            msg: "invalid console major",
        })?;
        let minor = parse::parse_dec_u32(minor).map_err(|_| Error::Parse {
            path: path.to_path_buf(),
            line: line_num,
            msg: "invalid console minor",
        })?;
        (Some(major), Some(minor))
    };

    Ok(Console {
        name: name.into(),
        flags,
        major,
        minor,
    })
}
