//! A minimal LZ4 block encoder for the tests: greedy, one candidate per four-byte key. It
//! produces valid blocks, which is all the decoder tests need.

use std::collections::HashMap;

const MINIMUM_MATCH: usize = 4;
const LAST_LITERALS: usize = 5;
const MAXIMUM_OFFSET: usize = 0xFFFF;

pub fn compress(input: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut table: HashMap<[u8; 4], usize> = HashMap::new();

    let mut anchor = 0;
    let mut position = 0;
    let limit = input.len().saturating_sub(LAST_LITERALS);

    while position < limit {
        let key = [
            input[position],
            input[position + 1],
            input[position + 2],
            input[position + 3],
        ];
        if let Some(&candidate) = table.get(&key) {
            let distance = position - candidate;
            if distance > 0
                && distance <= MAXIMUM_OFFSET
                && input[candidate..candidate + MINIMUM_MATCH]
                    == input[position..position + MINIMUM_MATCH]
            {
                let mut length = MINIMUM_MATCH;
                while position + length < limit
                    && input[candidate + length] == input[position + length]
                {
                    length += 1;
                }

                emit_sequence(&mut output, &input[anchor..position], distance, length);
                table.insert(key, position);
                position += length;
                anchor = position;
                continue;
            }
        }

        table.insert(key, position);
        position += 1;
    }

    emit_last_literals(&mut output, &input[anchor..]);
    output
}

fn emit_sequence(output: &mut Vec<u8>, literals: &[u8], offset: usize, match_length: usize) {
    let match_extra = match_length - MINIMUM_MATCH;
    output.push(token(literals.len().min(15), match_extra.min(15)));

    if literals.len() >= 15 {
        write_extra(output, literals.len() - 15);
    }
    output.extend_from_slice(literals);

    output.push(offset.to_le_bytes()[0]);
    output.push(offset.to_le_bytes()[1]);

    if match_extra >= 15 {
        write_extra(output, match_extra - 15);
    }
}

fn emit_last_literals(output: &mut Vec<u8>, literals: &[u8]) {
    output.push(token(literals.len().min(15), 0));
    if literals.len() >= 15 {
        write_extra(output, literals.len() - 15);
    }
    output.extend_from_slice(literals);
}

fn token(literals: usize, matched: usize) -> u8 {
    u8::try_from((literals << 4) | matched).unwrap()
}

fn write_extra(output: &mut Vec<u8>, mut remainder: usize) {
    while remainder >= 255 {
        output.push(255);
        remainder -= 255;
    }
    output.push(u8::try_from(remainder).unwrap());
}
