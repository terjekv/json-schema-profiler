use serde_json::Number;

/// Classify exact decimal spelling without expanding exponents or rounding through f64.
pub(crate) fn is_integer(number: &Number) -> bool {
    let token = number.as_str();
    let (mantissa, exponent) = token.split_once(['e', 'E']).unwrap_or((token, "0"));
    let mut fractional = 0_i64;
    let mut after_dot = false;
    let mut trailing_zeros = 0_i64;
    let mut nonzero = false;
    for byte in mantissa.bytes() {
        match byte {
            b'.' => after_dot = true,
            b'-' => (),
            digit => {
                fractional += i64::from(after_dot);
                if digit == b'0' {
                    trailing_zeros += 1;
                } else {
                    trailing_zeros = 0;
                    nonzero = true;
                }
            }
        }
    }
    if !nonzero {
        return true;
    }
    let negative = exponent.starts_with('-');
    let magnitude = exponent
        .trim_start_matches(['+', '-'])
        .bytes()
        .fold(0_i64, |value, byte| {
            value
                .saturating_mul(10)
                .saturating_add(i64::from(byte - b'0'))
        });
    let exponent = if negative { -magnitude } else { magnitude };
    exponent >= fractional - trailing_zeros
}
