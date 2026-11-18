//! JSON output that is byte-for-byte what the PHP application sends.
//!
//! Organizr writes every API response with
//! `json_encode($value, JSON_HEX_QUOT | JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES)`.
//! Clients may depend on that exact text, so this module reproduces it:
//!
//! - four-space indentation, `"key": value`, `[]` and `{}` for empty containers
//! - `"` inside a string is written as `"`; `/` is written as it is
//! - every character outside ASCII is written as `\uXXXX` in lower-case hexadecimal,
//!   with a surrogate pair for characters above U+FFFF
//! - a float with no fractional part is written without one (`10.0` becomes `10`)
//! - a float below `1.0e-4` or from `1.0e+17` upwards uses PHP's exponent form (`1.0e+25`)
//!
//! `parity/php-json/generate.php` records what real PHP prints. The tests compare against it.

use std::io;

use serde::Serialize;
use serde_json::ser::{CharEscape, Formatter, PrettyFormatter, Serializer};

/// Serialises `value` as PHP's `json_encode` does with the flags Organizr uses.
///
/// A float that is not finite is written as `null`. PHP refuses to encode such a value.
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, serde_json::Error>
where
    T: Serialize + ?Sized,
{
    let mut out = Vec::with_capacity(256);
    let mut serializer = Serializer::with_formatter(&mut out, PhpFormatter::new());
    value.serialize(&mut serializer)?;
    Ok(out)
}

/// PHP's float formatter uses exponent form from this many digits before the decimal point.
const MAX_INTEGER_DIGITS: i32 = 17;

/// PHP's float formatter uses exponent form when the first digit is further than this
/// behind the decimal point. `0.0001` has three zeros after the point and stays as it is.
const MAX_LEADING_ZEROS: i32 = 3;

/// A `serde_json` formatter that prints what PHP prints.
struct PhpFormatter {
    /// Supplies the layout: PHP's pretty printer and `serde_json`'s agree once the
    /// indentation is four spaces.
    layout: PrettyFormatter<'static>,
}

impl PhpFormatter {
    fn new() -> Self {
        Self {
            layout: PrettyFormatter::with_indent(b"    "),
        }
    }
}

impl Formatter for PhpFormatter {
    fn write_f32<W>(&mut self, writer: &mut W, value: f32) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        write_float(writer, &format!("{value:e}"))
    }

    fn write_f64<W>(&mut self, writer: &mut W, value: f64) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        write_float(writer, &format!("{value:e}"))
    }

    fn write_string_fragment<W>(&mut self, writer: &mut W, fragment: &str) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut rest = fragment;
        while let Some(position) = rest.find(|character: char| !character.is_ascii()) {
            let Some((ascii, tail)) = rest.split_at_checked(position) else {
                break;
            };
            writer.write_all(ascii.as_bytes())?;
            let mut characters = tail.chars();
            if let Some(character) = characters.next() {
                let mut units = [0_u16; 2];
                for unit in character.encode_utf16(&mut units) {
                    write!(writer, "\\u{unit:04x}")?;
                }
            }
            rest = characters.as_str();
        }
        writer.write_all(rest.as_bytes())
    }

    fn write_char_escape<W>(&mut self, writer: &mut W, escape: CharEscape) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        match escape {
            CharEscape::Quote => writer.write_all(b"\\u0022"),
            CharEscape::ReverseSolidus => writer.write_all(b"\\\\"),
            CharEscape::Solidus => writer.write_all(b"/"),
            CharEscape::Backspace => writer.write_all(b"\\b"),
            CharEscape::FormFeed => writer.write_all(b"\\f"),
            CharEscape::LineFeed => writer.write_all(b"\\n"),
            CharEscape::CarriageReturn => writer.write_all(b"\\r"),
            CharEscape::Tab => writer.write_all(b"\\t"),
            CharEscape::AsciiControl(byte) => write!(writer, "\\u{byte:04x}"),
        }
    }

    fn begin_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.layout.begin_array(writer)
    }

    fn end_array<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.layout.end_array(writer)
    }

    fn begin_array_value<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.layout.begin_array_value(writer, first)
    }

    fn end_array_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.layout.end_array_value(writer)
    }

    fn begin_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.layout.begin_object(writer)
    }

    fn end_object<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.layout.end_object(writer)
    }

    fn begin_object_key<W>(&mut self, writer: &mut W, first: bool) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.layout.begin_object_key(writer, first)
    }

    fn begin_object_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.layout.begin_object_value(writer)
    }

    fn end_object_value<W>(&mut self, writer: &mut W) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        self.layout.end_object_value(writer)
    }
}

/// Writes a finite float the way PHP does with `serialize_precision = -1`.
///
/// `scientific` is Rust's `{:e}` form of the float, for example `-1.2345e-7`. Rust and PHP
/// both use the shortest digits that read back as the same float, so only the layout differs.
fn write_float<W>(writer: &mut W, scientific: &str) -> io::Result<()>
where
    W: ?Sized + io::Write,
{
    let (negative, unsigned) = match scientific.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, scientific),
    };
    let (mantissa, exponent) = unsigned
        .split_once('e')
        .ok_or_else(|| invalid_float(scientific))?;
    let exponent: i32 = exponent.parse().map_err(|_| invalid_float(scientific))?;
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let (first, later) = match digits.split_at_checked(1) {
        Some((first, later)) if !first.is_empty() => (first, later),
        _ => return Err(invalid_float(scientific)),
    };

    // The number of digits before the decimal point. Zero or less means the value is
    // below one and has that many zeros between the point and its first digit.
    let point = exponent.saturating_add(1);
    let uses_exponent_form = if point < 0 {
        point < -MAX_LEADING_ZEROS
    } else {
        point > MAX_INTEGER_DIGITS
    };

    if negative {
        writer.write_all(b"-")?;
    }
    if uses_exponent_form {
        let fraction = if later.is_empty() { "0" } else { later };
        let sign = if exponent < 0 { '-' } else { '+' };
        write!(
            writer,
            "{first}.{fraction}e{sign}{}",
            exponent.unsigned_abs()
        )
    } else if point <= 0 {
        let zeros = "0".repeat(usize::try_from(point.unsigned_abs()).unwrap_or(0));
        write!(writer, "0.{zeros}{digits}")
    } else {
        let whole_length = usize::try_from(point).unwrap_or(0);
        match digits.split_at_checked(whole_length) {
            Some((whole, fraction)) if !fraction.is_empty() => write!(writer, "{whole}.{fraction}"),
            _ => write!(writer, "{digits:0<whole_length$}"),
        }
    }
}

/// The error for a float whose `{:e}` form is not `digits[.digits]e[-]digits`.
fn invalid_float(scientific: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("cannot format the float `{scientific}`"),
    )
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use super::to_vec;

    fn encode<T: Serialize + ?Sized>(value: &T) -> String {
        String::from_utf8(to_vec(value).expect("the value can be encoded"))
            .expect("the writer produces UTF-8")
    }

    #[test]
    fn struct_fields_keep_their_declared_order() {
        #[derive(Serialize)]
        struct Response {
            result: &'static str,
            message: Option<&'static str>,
            data: Vec<u8>,
        }

        let response = Response {
            result: "success",
            message: None,
            data: Vec::new(),
        };

        assert_eq!(
            encode(&response),
            "{\n    \"result\": \"success\",\n    \"message\": null,\n    \"data\": []\n}"
        );
    }

    #[test]
    fn single_precision_float_uses_its_own_shortest_digits() {
        assert_eq!(encode(&0.1_f32), "0.1");
        assert_eq!(encode(&16_777_216_f32), "16777216");
    }

    #[test]
    fn float_that_is_not_finite_becomes_null() {
        assert_eq!(encode(&f64::NAN), "null");
        assert_eq!(encode(&f64::INFINITY), "null");
        assert_eq!(encode(&f64::NEG_INFINITY), "null");
    }
}
