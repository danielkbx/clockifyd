use std::io::{self, BufRead, Write};

use crate::error::CfdError;

pub fn confirm(prompt: &str) -> Result<bool, CfdError> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    confirm_with_reader(prompt, &mut reader)
}

pub(crate) fn prompt_line_with_io(
    prompt: &str,
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
) -> Result<String, CfdError> {
    write!(writer, "{prompt}")?;
    writer.flush()?;

    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn confirm_with_reader(prompt: &str, reader: &mut dyn BufRead) -> Result<bool, CfdError> {
    eprint!("{prompt} [y/N]: ");
    io::stderr().flush()?;

    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(matches!(line.trim(), "y" | "Y" | "yes" | "YES"))
}

pub(crate) fn select_index_with_io(
    prompt: &str,
    max_index: usize,
    default_index: usize,
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
) -> Result<usize, CfdError> {
    loop {
        let answer = prompt_line_with_io(prompt, reader, writer)?;
        if answer.is_empty() {
            return Ok(default_index);
        }

        match answer.parse::<usize>() {
            Ok(index) if index <= max_index => return Ok(index),
            _ => {
                writeln!(
                    writer,
                    "invalid selection; choose a number between 0 and {max_index}"
                )?;
                writer.flush()?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn confirm_accepts_yes_variants() {
        let mut reader = Cursor::new("yes\n");
        assert!(confirm_with_reader("Proceed?", &mut reader).unwrap());
    }

    #[test]
    fn confirm_defaults_to_no() {
        let mut reader = Cursor::new("\n");
        assert!(!confirm_with_reader("Proceed?", &mut reader).unwrap());
    }

    #[test]
    fn prompt_line_trims_input() {
        let mut reader = Cursor::new("  secret  \n");
        let mut writer = Vec::new();

        let value = prompt_line_with_io("API key: ", &mut reader, &mut writer).unwrap();

        assert_eq!(value, "secret");
        assert_eq!(String::from_utf8(writer).unwrap(), "API key: ");
    }

    #[test]
    fn select_index_accepts_default_none() {
        let mut reader = Cursor::new("\n");
        let mut writer = Vec::new();

        let value = select_index_with_io("Select [0]: ", 3, 0, &mut reader, &mut writer).unwrap();

        assert_eq!(value, 0);
    }

    #[test]
    fn select_index_accepts_non_zero_default() {
        let mut reader = Cursor::new("\n");
        let mut writer = Vec::new();

        let value = select_index_with_io("Select [2]: ", 3, 2, &mut reader, &mut writer).unwrap();

        assert_eq!(value, 2);
    }

    #[test]
    fn select_index_retries_until_valid() {
        let mut reader = Cursor::new("9\n2\n");
        let mut writer = Vec::new();

        let value = select_index_with_io("Select [0]: ", 3, 0, &mut reader, &mut writer).unwrap();
        let output = String::from_utf8(writer).unwrap();

        assert_eq!(value, 2);
        assert!(output.contains("invalid selection"));
    }
}
