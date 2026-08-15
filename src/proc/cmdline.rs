use crate::error::Result;
use crate::util::parse;

/// Reads the kernel boot command line from `/proc/cmdline`.
///
/// Returns the boot parameters as a list of space-separated arguments,
/// each either a `key=value` pair (e.g. `root=UUID=...`) or a bare
/// flag (e.g. `rw`, `quiet`). The trailing newline is stripped.
///
/// The kernel stores the string passed by the bootloader verbatim and
/// does not re-parse quoting. Bootloaders such as GRUB may therefore
/// embed literal `"..."` sections whose inner spaces belong to a single
/// argument. This function treats a quoted section as one token and
/// strips the quote characters, matching the bootloader's intent.
pub fn cmdline() -> Result<Vec<String>> {
    let bytes = parse::read_file(std::path::Path::new("/proc/cmdline"))?;
    Ok(parse_cmdline(parse::trim_end(&bytes)))
}

/// Splits a boot command line into arguments.
///
/// Whitespace separates arguments unless it appears inside a `"` or
/// `'` quoted section. A backslash escapes the following character.
/// An unterminated quote is not an error: the rest of the line is
/// treated as a single argument.
fn parse_cmdline(bytes: &[u8]) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = Vec::new();
    let mut in_quote = false;

    let mut iter = bytes.iter();
    while let Some(&b) = iter.next() {
        match b {
            b'"' | b'\'' => in_quote = !in_quote,
            b' ' | b'\t' if !in_quote => {
                if !cur.is_empty() {
                    args.push(String::from_utf8_lossy(&cur).into_owned());
                    cur.clear();
                }
            }
            b'\\' => {
                if let Some(&next) = iter.next() {
                    cur.push(next);
                }
            }
            _ => cur.push(b),
        }
    }

    if !cur.is_empty() {
        args.push(String::from_utf8_lossy(&cur).into_owned());
    }

    args
}
