//! The LZ4 block format (public specification), decoded into a buffer of known size.

use super::UnityAssetFormatError;

const MINIMUM_MATCH: usize = 4;

/// Decodes one LZ4 block into `destination` and returns the number of bytes written.
///
/// # Errors
///
/// The block is truncated, refers outside what it has decoded, or decodes to more than
/// `destination` can hold.
pub fn decode(source: &[u8], destination: &mut [u8]) -> Result<usize, UnityAssetFormatError> {
    let mut input = 0;
    let mut output = 0;

    while input < source.len() {
        let token = usize::from(source[input]);
        input += 1;

        let mut literals = token >> 4;
        if literals == 15 {
            literals = literals.saturating_add(read_extra_length(source, &mut input)?);
        }

        if literals > 0 {
            let available = source.len() - input;
            if literals > available {
                return Err(truncated("literals", literals, available));
            }
            let room = destination.len() - output;
            if literals > room {
                return Err(too_large(
                    output.saturating_add(literals),
                    destination.len(),
                ));
            }
            destination[output..output + literals]
                .copy_from_slice(&source[input..input + literals]);
            input += literals;
            output += literals;
        }

        // The last sequence of a block ends on its literals: no offset, no match.
        if input == source.len() {
            break;
        }

        if source.len() - input < 2 {
            return Err(truncated("the match offset", 2, source.len() - input));
        }
        let offset = usize::from(source[input]) | (usize::from(source[input + 1]) << 8);
        input += 2;

        if offset == 0 || offset > output {
            return Err(UnityAssetFormatError::new(format!(
                "the LZ4 block refers {offset} bytes back while only {output} bytes have been decoded."
            )));
        }

        let mut length = token & 0x0F;
        if length == 15 {
            length = length.saturating_add(read_extra_length(source, &mut input)?);
        }
        length = length.saturating_add(MINIMUM_MATCH);
        if length > destination.len() - output {
            return Err(too_large(output.saturating_add(length), destination.len()));
        }

        // Byte by byte on purpose: a match may overlap itself, which is how the format
        // encodes runs (offset 1 repeats one byte).
        let from = output - offset;
        for index in 0..length {
            destination[output + index] = destination[from + index];
        }
        output += length;
    }

    Ok(output)
}

fn read_extra_length(source: &[u8], input: &mut usize) -> Result<usize, UnityAssetFormatError> {
    let mut extra = 0_usize;
    loop {
        let Some(value) = source.get(*input).copied() else {
            return Err(truncated("a length byte", 1, 0));
        };
        *input += 1;
        extra = extra.saturating_add(usize::from(value));
        if value != 0xFF {
            return Ok(extra);
        }
    }
}

fn truncated(what: &str, wanted: usize, available: usize) -> UnityAssetFormatError {
    UnityAssetFormatError::new(format!(
        "the LZ4 block ends early: {what} needs {wanted} byte(s), {available} left."
    ))
}

fn too_large(wanted: usize, capacity: usize) -> UnityAssetFormatError {
    UnityAssetFormatError::new(format!(
        "the LZ4 block decodes to more than the announced {capacity} bytes (at least {wanted})."
    ))
}
