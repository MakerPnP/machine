#![no_std]

#[cfg(feature = "std")]
extern crate alloc;
#[cfg(feature = "std")]
use alloc::string::String;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MorseSymbol {
    IntraLetter = 0b00,
    Dit   = 0b01,
    Dash  = 0b10,
    IntraWord = 0b11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MorseCharacter {
    Character(char),
    IntraWord,
    Stop,
}

impl MorseSymbol {
    #[inline]
    pub const fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0b00 => MorseSymbol::IntraLetter,
            0b01 => MorseSymbol::Dit,
            0b10 => MorseSymbol::Dash,
            0b11 => MorseSymbol::IntraWord,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MorseBitstream {
    pub bytes: [u8; 256],
    pub symbol_count: usize,
}

impl MorseBitstream {
    pub const fn new() -> Self {
        Self {
            bytes: [0; 256],
            symbol_count: 0,
        }
    }

    pub fn push(&mut self, symbol: MorseSymbol) {
        let byte_idx = self.symbol_count / 4;
        let bit_shift = (self.symbol_count % 4) * 2;

        // Isolate and wipe out the 2-bit slot
        self.bytes[byte_idx] &= !(0b11 << bit_shift);
        // Pack the new symbol bits into place
        self.bytes[byte_idx] |= (symbol as u8) << bit_shift;

        self.symbol_count += 1;
    }

    pub fn iter(&self) -> MorseIterator<'_> {
        MorseIterator {
            bitstream: self,
            current_idx: 0,
        }
    }

    pub fn iter_characters(&self) -> MorseCharacterIterator<'_> {
        MorseCharacterIterator::new(self.iter())
    }

    pub fn fold<B, F>(&self, init: B, mut f: F) -> B
    where
        F: FnMut(B, MorseSymbol) -> B,
    {
        let mut accum = init;
        let mut idx = 0;
        while idx < self.symbol_count {
            let byte_idx = idx / 4;
            let bit_shift = (idx % 4) * 2;
            let bits = (self.bytes[byte_idx] >> bit_shift) & 0b11;
            accum = f(accum, MorseSymbol::from_bits(bits));
            idx += 1;
        }
        accum
    }

    pub fn fold_characters<B, F>(&self, init: B, f: F) -> B
    where
        F: FnMut(B, MorseCharacter) -> B,
    {
        self.iter_characters().fold(init, f)
    }
}

pub struct MorseIterator<'a> {
    bitstream: &'a MorseBitstream,
    current_idx: usize,
}

impl<'a> Iterator for MorseIterator<'a> {
    type Item = MorseSymbol;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_idx >= self.bitstream.symbol_count {
            return None;
        }
        let byte_idx = self.current_idx / 4;
        let bit_shift = (self.current_idx % 4) * 2;
        let bits = (self.bitstream.bytes[byte_idx] >> bit_shift) & 0b11;

        self.current_idx += 1;
        Some(MorseSymbol::from_bits(bits))
    }
}

/// A high-level iterator that consumes low-level MorseSymbols
/// and transforms them into complete alphanumeric characters or gaps.
pub struct MorseCharacterIterator<'a> {
    symbol_iter: MorseIterator<'a>,
    current_sequence: [MorseSymbol; 8],
    seq_len: usize,
    sent_stop: bool,
}

impl<'a> MorseCharacterIterator<'a> {
    pub fn new(symbol_iter: MorseIterator<'a>) -> Self {
        Self {
            symbol_iter,
            current_sequence: [MorseSymbol::IntraLetter; 8],
            seq_len: 0,
            sent_stop: false,
        }
    }

    /// Internal helper to check our symbol lookup sheet for an assembled character match
    #[inline]
    fn flush_current_character(&mut self) -> Option<MorseCharacter> {
        if self.seq_len > 0 {
            let match_result = MORSE_TABLE
                .iter()
                .find(|(_, syms)| *syms == &self.current_sequence[..self.seq_len])
                .map(|(ch, _)| MorseCharacter::Character(*ch));
            self.seq_len = 0;
            match_result
        } else {
            None
        }
    }
}

impl<'a> Iterator for MorseCharacterIterator<'a> {
    type Item = MorseCharacter;

    fn next(&mut self) -> Option<Self::Item> {
        if self.sent_stop {
            return None;
        }

        loop {
            match self.symbol_iter.next() {
                Some(symbol @ MorseSymbol::Dit) | Some(symbol @ MorseSymbol::Dash) => {
                    if self.seq_len < self.current_sequence.len() {
                        self.current_sequence[self.seq_len] = symbol;
                        self.seq_len += 1;
                    }
                }
                Some(MorseSymbol::IntraLetter) => {
                    if let Some(character_token) = self.flush_current_character() {
                        return Some(character_token);
                    }
                    // If no character was aggregated, continue loop to skip redundant spaces
                }
                Some(MorseSymbol::IntraWord) => {
                    if let Some(character_token) = self.flush_current_character() {
                        // We found a trailing character before this word boundary pause.
                        // We return the character token now. The next next() invocation
                        // will fall through into an empty buffer state and yield the Pause.
                        // To preserve the Pause token, we manually step back the iterator index
                        // or let the next loop run hit the natural empty sequence catch.
                        self.symbol_iter.current_idx -= 1;
                        return Some(character_token);
                    }
                    return Some(MorseCharacter::IntraWord);
                }
                None => {
                    // Symbol stream ran dry. Check if there's an unflushed trailing character.
                    if let Some(character_token) = self.flush_current_character() {
                        return Some(character_token);
                    }
                    // Finalize stream sequence lifecycle
                    self.sent_stop = true;
                    return Some(MorseCharacter::Stop);
                }
            }
        }
    }
}

pub const MORSE_TABLE: &[(char, &[MorseSymbol])] = &[
    // --- Alphanumeric ---
    ('A', &[MorseSymbol::Dit, MorseSymbol::Dash]),
    ('B', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit]),
    ('C', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit]),
    ('D', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit]),
    ('E', &[MorseSymbol::Dit]),
    ('F', &[MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit]),
    ('G', &[MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit]),
    ('H', &[MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit]),
    ('I', &[MorseSymbol::Dit, MorseSymbol::Dit]),
    ('J', &[MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash]),
    ('K', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash]),
    ('L', &[MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit]),
    ('M', &[MorseSymbol::Dash, MorseSymbol::Dash]),
    ('N', &[MorseSymbol::Dash, MorseSymbol::Dit]),
    ('O', &[MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash]),
    ('P', &[MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit]),
    ('Q', &[MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash]),
    ('R', &[MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit]),
    ('S', &[MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit]),
    ('T', &[MorseSymbol::Dash]),
    ('U', &[MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash]),
    ('V', &[MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash]),
    ('W', &[MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash]),
    ('X', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash]),
    ('Y', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash]),
    ('Z', &[MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit]),
    ('1', &[MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash]),
    ('2', &[MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash]),
    ('3', &[MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash]),
    ('4', &[MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash]),
    ('5', &[MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit]),
    ('6', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit]),
    ('7', &[MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit]),
    ('8', &[MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit]),
    ('9', &[MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit]),
    ('0', &[MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash]),

    // --- Full Punctuation & Symbols Set ---
    ('.', &[MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash]), // AAA
    (',', &[MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash]), // MIM
    ('?', &[MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit]), // IMI
    ('\'', &[MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit]), // WG
    ('!', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash]), // MN / KW
    ('/', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit]), // DN
    ('(', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit]), // KN
    (')', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash]), // KK
    ('&', &[MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit]), // AS
    (':', &[MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit]), // OS
    (';', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit]), // KR
    ('=', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash]), // BT
    ('+', &[MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit]), // AR
    ('-', &[MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash]), // DU
    ('_', &[MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash]), // IQ
    ('"', &[MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit]), // RR
    ('$', &[MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dit, MorseSymbol::Dash]), // SX
    ('@', &[MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dash, MorseSymbol::Dit, MorseSymbol::Dash, MorseSymbol::Dit]), // AC
];

pub fn encode_text(text: &str) -> MorseBitstream {
    let mut bitstream = MorseBitstream::new();
    let mut word_started = false;
    let mut need_char_space = false;

    for c in text.chars() {
        if c == ' ' {
            if word_started {
                bitstream.push(MorseSymbol::IntraWord);
                need_char_space = false;
            }
            continue;
        }

        let upper_c = c.to_ascii_uppercase();
        if let Some((_, symbols)) = MORSE_TABLE.iter().find(|(ch, _)| *ch == upper_c) {
            if need_char_space {
                bitstream.push(MorseSymbol::IntraLetter);
            }
            for &symbol in *symbols {
                bitstream.push(symbol);
            }
            word_started = true;
            need_char_space = true;
        }
    }
    bitstream
}

#[cfg(feature = "std")]
pub fn decode_text(bitstream: &MorseBitstream) -> String {
    let mut result = String::new();

    for char_token in bitstream.iter_characters() {
        match char_token {
            MorseCharacter::Character(ch) => result.push(ch),
            MorseCharacter::IntraWord => result.push(' '),
            MorseCharacter::Stop => break,
        }
    }
    result
}