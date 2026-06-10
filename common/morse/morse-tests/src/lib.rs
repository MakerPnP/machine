
#[cfg(test)]
mod tests {
    extern crate std;
    use std::string::String;
    use std::vec::Vec;
    use morse_core::{MorseCharacter, MorseBitstream, MorseSymbol, encode_text, decode_text};
    use morse_macro::morse;

    const TEST_MAP: &[(char, &str)] = &[
        // --- Alphanumeric ---
        ('A', ".-"),    ('B', "-..."),  ('C', "-.-."),  ('D', "-.."),
        ('E', "."),     ('F', "..-."),  ('G', "--."),   ('H', "...."),
        ('I', ".."),    ('J', ".---"),  ('K', "-.-"),   ('L', ".-.."),
        ('M', "--"),    ('N', "-."),    ('O', "---"),   ('P', ".--."),
        ('Q', "--.-"),  ('R', ".-."),   ('S', "..."),   ('T', "-"),
        ('U', "..-"),   ('V', "...-"),  ('W', ".--"),   ('X', "-..-"),
        ('Y', "-.--"),  ('Z', "--.."),
        ('1', ".----"), ('2', "..---"), ('3', "...--"), ('4', "....-"),
        ('5', "....."), ('6', "-...."), ('7', "--..."), ('8', "---.."),
        ('9', "----."), ('0', "-----"),

        // --- Punctuation & Symbols Set ---
        ('.', ".-.-.-"),
        (',', "--..--"),
        ('?', "..--.."),
        ('\'', ".----."),
        ('!', "-.-.--"),
        ('/', "-..-."),
        ('(', "-.--."),
        (')', "-.--.-"),
        ('&', ".-..."),
        (':', "---..."),
        (';', "-.-.-."),
        ('=', "-...-"),
        ('+', ".-.-."),
        ('-', "-....-"),
        ('_', "..--.-"),
        ('"', ".-..-."),
        ('$', "...-..-"),
        ('@', ".--.-."),
    ];

    fn encode_to_bytes(text: &str) -> (Vec<u8>, usize) {
        let mut raw_symbols = Vec::new();
        let mut word_started = false;
        let mut need_space = false;

        for c in text.chars() {
            if c == ' ' {
                if word_started {
                    raw_symbols.push(0b11u8); // Intra-word
                    need_space = false;
                }
                continue;
            }
            let upper = c.to_ascii_uppercase();
            if let Some((_, pattern)) = TEST_MAP.iter().find(|(ch, _)| *ch == upper) {
                if need_space {
                    raw_symbols.push(0b00u8); // Space
                }
                for sym_char in pattern.chars() {
                    match sym_char {
                        '.' => raw_symbols.push(0b01u8), // Dit
                        '-' => raw_symbols.push(0b10u8), // Dash
                        _ => {}
                    }
                }
                word_started = true;
                need_space = true;
            }
        }

        let mut packed_bytes = Vec::new();
        for chunk in raw_symbols.chunks(4) {
            let mut byte = 0u8;
            for (i, &val) in chunk.iter().enumerate() {
                byte |= val << (i * 2);
            }
            packed_bytes.push(byte);
        }

        (packed_bytes, raw_symbols.len())
    }

    /// Encode and verify bitstream using the independent packing algorithm
    #[test]
    fn test_encode() {
        let input = "Hello world";
        let runtime_stream = encode_text(input);
        let (expected_bytes, expected_count) = encode_to_bytes(input);

        assert_eq!(runtime_stream.symbol_count, expected_count);

        for (i, &expected_byte) in expected_bytes.iter().enumerate() {
            assert_eq!(runtime_stream.bytes[i], expected_byte, "Byte mismatch at index {}", i);
        }
    }

    /// Encode and decode
    #[test]
    fn test_decode_hello_world() {
        let original = "HELLO WORLD";
        let encoded = encode_text(original);
        let decoded = decode_text(&encoded);

        assert_eq!(original, decoded);
    }

    /// Round trip every possible ASCII character without touching production tables
    #[test]
    fn test_round_trip() {
        for &(ch, expected_pattern) in TEST_MAP {
            let mut text_str = String::new();
            text_str.push(ch);

            // Encode using production runtime
            let encoded = encode_text(&text_str);

            // Reconstruct the character purely using our independent iterator interpreter
            let mut manual_pattern = String::new();
            for symbol in encoded.iter() {
                match symbol {
                    MorseSymbol::Dit => manual_pattern.push('.'),
                    MorseSymbol::Dash => manual_pattern.push('-'),
                    MorseSymbol::IntraLetter | MorseSymbol::IntraWord => {}
                }
            }

            assert_eq!(expected_pattern, manual_pattern, "Failed on character: {}", ch);
        }
    }

    #[test]
    fn test_macro_compile_time_generation() {
        let macro_stream: MorseBitstream = morse!("Hello world");
        let (expected_bytes, expected_count) = encode_to_bytes("Hello world");

        assert_eq!(macro_stream.symbol_count, expected_count);
        for (i, &expected_byte) in expected_bytes.iter().enumerate() {
            assert_eq!(macro_stream.bytes[i], expected_byte);
        }
    }

    #[test]
    fn test_macro_fold_to_exclamation_string() {
        // 1. Generate the bitstream at compile time using the macro
        let macro_stream: MorseBitstream = morse!("FOLD-ME");

        // 2. Fold over semantic characters instead of raw bits
        let result = macro_stream.fold_characters(String::from("!"), |mut acc, morse_char| {
            match morse_char {
                MorseCharacter::Character(c) => acc.push(c.to_ascii_lowercase()),
                MorseCharacter::IntraWord => acc.push(' '),
                MorseCharacter::Stop         => acc.push('!'), // End of stream wrapping
            }
            acc
        });

        // 3. Verify the result
        assert_eq!(result, "!fold-me!");
    }
}
