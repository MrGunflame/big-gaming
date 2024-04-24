use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum FromHexError {
    #[error("invalid length")]
    InvalidLength,
    #[error("bad hex character {0} at {1}")]
    BadCharacter(u8, usize),
}

/// Decodes a `str` into a constant-sized array with length `N`.
pub const fn decode_to_array<const N: usize>(input: &str) -> Result<[u8; N], FromHexError> {
    if input.len() != N * 2 {
        return Err(FromHexError::InvalidLength);
    }

    let mut output = [0; N];

    // for not allowed in const fn.
    let mut index = 0;
    while index < input.len() {
        let char = input.as_bytes()[index];

        let mut nibble = match char {
            b'0'..=b'9' => char - b'0',
            b'a'..=b'f' => char - b'a' + 10,
            b'A'..=b'F' => char - b'A' + 10,
            _ => return Err(FromHexError::BadCharacter(char, index)),
        };

        if index % 2 == 0 {
            nibble <<= 4;
        }

        output[index / 2] |= nibble;
        index += 1;
    }

    Ok(output)
}
