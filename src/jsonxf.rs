//! Jsonxf is a JSON transformer, providing fast pretty-printing and
//! minimizing of JSON-encoded data.
//!
//! It's designed for speed, and does *not* perform JSON validation.
//! Jsonxf is not guaranteed to either accept or reject invalid input.
//!


use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Error;

/// Pretty-prints a stream of JSON-formatted data.
///
/// This function does *not* perform JSON validation, and is not guaranteed
/// to either reject or accept invalid input.
///
/// The `indent` parameter sets the string used to indent pretty-printed
/// output; e.g., `"  "` or `"\t"`.
///
/// `pretty_print_stream` uses `std::io::BufReader` and `std::io:BufWriter`
/// to provide IO buffering; no external buffering should be necessary.
///
/// ```
pub fn pretty_print_stream(input: &mut Read, output: &mut Write, indent: &str) -> Result<(), Error> {
  /*
    Strategy: pass bytes from `input` to `output`, taking notice of when the
    current byte is:

    * Part of a JSON string (and if so, whether it follows a backslash)

    * One of `{`, `}`, `[`, or `]`, which affect indent level and usually
      emit whitespace

    * `,`, which does not affect indent level but always emits whitespace

    Empty JSON structures `{}` and `[]` are treated as special cases and
    rendered as such.
  */

  let reader = BufReader::new(input);
  let mut writer = BufWriter::new(output);

  let mut depth = 0;

  // string special cases
  let mut in_string = false;
  let mut in_backslash = false;

  // whitespace special cases, after { or [
  let mut whitespace_buffer = String::from("");
  let mut current_structure_is_empty = false;

  for byte in reader.bytes() {
    let b = byte?;
    let c = b as char;

    if in_string {
      writer.write(&[b])?;
      if in_backslash { in_backslash = false; }
      else if c == '"' { in_string = false; }
      else if c == '\\' { in_backslash = true; }
    }
    else {
      match c {
        ' ' | '\t' | '\n' | '\r' => {
          /* skip whitespace */
        },

        '{' | '[' => {
          writer.write(&[b])?;
          depth += 1;
          // don't write trailing whitespace immediately, in case this
          // is an empty structure
          current_structure_is_empty = true;
          whitespace_buffer = String::from("\n");
          for _ in 0..depth {
            whitespace_buffer.push_str(indent);
          }
        },

        '}' | ']' => {
          depth -= 1;
          if current_structure_is_empty {
            writer.write(&[b])?;
            current_structure_is_empty = false;
          }
          else {
            writer.write(&['\n' as u8])?;
            for _ in 0..depth {
              writer.write(indent.as_bytes())?;
            }
            writer.write(&[b])?;
          }
          if depth == 0 {
            writer.write(&['\n' as u8])?;
          }
        },

        ',' => {
          writer.write(&[b, '\n' as u8])?;
          for _ in 0..depth {
            writer.write(indent.as_bytes())?;
          }
        },

        ':' => {
          writer.write(&[':' as u8, ' ' as u8])?;
        },

        c => {
          if current_structure_is_empty {
            writer.write(whitespace_buffer.as_bytes())?;
            current_structure_is_empty = false;
          }
          if c == '"' {
            in_string = true;
          }
          writer.write(&[b])?;
        },
      }
    }
  }
  return Ok(());
}

/// Returns a pretty-printed representation of the given string containing
/// JSON-encoded data.
///
/// This function does *not* perform JSON validation, and is not guaranteed
/// to either reject or accept invalid input.
///
/// The `indent` parameter sets the string used to indent pretty-printed
/// output; e.g., `"  "` or `"\t"`.
pub fn pretty_print(json_string: &str, indent: &str) -> Result<String, String> {
  let mut input = json_string.as_bytes();
  let mut output: Vec<u8> = vec![];
  match pretty_print_stream(&mut input, &mut output, indent) {
    Ok(_) => {},
    Err(f) => { return Err(f.to_string()); },
  };
  let output_string = match String::from_utf8(output) {
    Ok(s) => { s },
    Err(f) => { return Err(f.to_string()); },
  };
  return Ok(output_string);
}



/// Minimizes JSON-formatted input.
///
/// This function does *not* perform JSON validation.  It works on
/// valid JSON, and performs best-effort formatting of invalid JSON,
/// but it is not guaranteed to flag or tolerate bad input.
///
/// `minimize` uses `std::io::BufReader` and `std::io:BufWriter` to
/// provide IO buffering; no external buffering should be necessary.
///
pub fn minimize_stream(input: &mut Read, output: &mut Write) -> Result<(), Error> {
  // Strategy is similar to `pretty_print`, with much less whitespace mgmt.
  // Care is taken not to emit a newline at the end of the stream.

  let reader = BufReader::new(input);
  let mut writer = BufWriter::new(output);

  let mut in_string = false;
  let mut in_backslash = false;
  let mut depth = 0;
  let mut print_newline = false;

  for byte in reader.bytes() {
    let b = byte?;
    let c = b as char;

    if in_string {
      writer.write(&[b])?;
      if in_backslash { in_backslash = false; }
      else if c == '"' { in_string = false; }
      else if c == '\\' { in_backslash = true; }
    }
    else {
      match c {
        ' ' | '\t' | '\n' | '\r' => {
          /* skip whitespace */
         },

        '{' | '[' => {
          if depth == 0 {
            if print_newline {
              writer.write(&['\n' as u8])?;
            }
            print_newline = true;
          }
          writer.write(&[b])?;
          depth += 1;
        },

        '}' | ']' => {
          writer.write(&[b])?;
          depth -= 1;
          //if depth == 0 {
            //writer.write(&['\n' as u8])?;
          //}
        },

        c => {
          if c == '"' {
            in_string = true;
          }
          writer.write(&[b])?;
        },
      }
    }
  }
  return Ok(());
}

/// Returns a minimized version of the given string containing JSON-
/// encoded data.
///
/// This function does *not* perform JSON validation.  It works on
/// valid JSON, and performs best-effort formatting of invalid JSON,
/// but it is not guaranteed to flag or tolerate bad input.
///
/// # Examples:
///
/// ```
/// assert_eq!(r#"{"a":"b","c":0}"#, jsonxf::minimize(r#"{ "a": "b", "c": 0 } "#).unwrap());
/// assert_eq!("null", jsonxf::minimize("\r\n\tnull\r\n").unwrap());
/// ```
pub fn minimize(json_string: &str) -> Result<String, String> {
  let mut input = json_string.as_bytes();
  let mut output: Vec<u8> = vec![];
  match minimize_stream(&mut input, &mut output) {
    Ok(_) => {},
    Err(f) => { return Err(f.to_string()); },
  };
  let output_string = match String::from_utf8(output) {
    Ok(s) => { s },
    Err(f) => { return Err(f.to_string()); },
  };
  return Ok(output_string);
}

