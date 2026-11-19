//! Detached EGG encryption records and per-file bounded cipher state.
use crate::{ArchiveError, OperationControl, Result};
use hmac::Mac;
use std::io::{self, Read};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

type Authenticator = hmac::Hmac<sha1::Sha1>;
#[derive(Clone, Copy)]
pub(super) struct Header {
    method: u8,
    data: [u8; 28],
}
impl Header {
    pub(super) fn length(method: u8) -> Result<u64> {
        match method {
            0 => Ok(17),
            1 | 5 => Ok(21),
            2 | 6 => Ok(29),
            _ => Err(ArchiveError::UnsupportedEncryption),
        }
    }
    pub(super) fn read(input: &mut impl Read) -> Result<Self> {
        let method = super::exact::<1>(input)?[0];
        let length = Self::length(method)? as usize - 1;
        let mut data = [0; 28];
        input
            .read_exact(&mut data[..length])
            .map_err(super::input_error)?;
        Ok(Self { method, data })
    }
    pub(super) fn tag(&self) -> Option<&[u8]> {
        let offset = match self.method {
            1 | 5 => 10,
            2 | 6 => 18,
            _ => return None,
        };
        Some(&self.data[offset..offset + 10])
    }
}

enum Cipher {
    Traditional(crate::formats::zip_crypto::Keys),
    Aes(Box<crate::formats::zip_aes::Cipher>),
    Lea(Box<Lea>),
}
pub(super) struct State {
    cipher: Cipher,
    position: u64,
}
impl State {
    pub(super) fn new(
        header: &Header,
        password: &str,
        control: &OperationControl,
    ) -> Result<(Self, Option<Authenticator>)> {
        control.checkpoint()?;
        if header.method == 0 {
            let mut keys = crate::formats::zip_crypto::initial_keys(password.as_bytes());
            let mut verifier = Zeroizing::new([0; 12]);
            verifier.copy_from_slice(&header.data[..12]);
            crate::formats::zip_crypto::decrypt(&mut keys, &mut *verifier);
            if verifier[11] != header.data[15] {
                return Err(ArchiveError::AuthenticationOrDataError);
            }
            return Ok((
                Self {
                    cipher: Cipher::Traditional(keys),
                    position: 0,
                },
                None,
            ));
        }
        let key_length = if matches!(header.method, 1 | 5) {
            16
        } else {
            32
        };
        let salt_length = key_length / 2;
        let mut derived = Zeroizing::new([0; 66]);
        pbkdf2::pbkdf2_hmac::<sha1::Sha1>(
            password.as_bytes(),
            &header.data[..salt_length],
            1000,
            &mut derived[..2 * key_length + 2],
        );
        control.checkpoint()?;
        if !bool::from(
            derived[2 * key_length..2 * key_length + 2]
                .ct_eq(&header.data[salt_length..salt_length + 2]),
        ) {
            return Err(ArchiveError::AuthenticationOrDataError);
        }
        let mac = <Authenticator as Mac>::new_from_slice(&derived[key_length..2 * key_length])
            .map_err(|_| ArchiveError::UnsupportedEncryption)?;
        let cipher = match header.method {
            1 | 2 => Cipher::Aes(Box::new(crate::formats::zip_aes::Cipher::new(
                if key_length == 16 { 1 } else { 3 },
                &derived[..key_length],
            )?)),
            5 | 6 => Cipher::Lea(Box::new(Lea::new(&derived[..key_length]))),
            _ => return Err(ArchiveError::UnsupportedEncryption),
        };
        Ok((
            Self {
                cipher,
                position: 0,
            },
            Some(mac),
        ))
    }
    fn decrypt(&mut self, bytes: &mut [u8]) {
        if let Cipher::Traditional(keys) = &mut self.cipher {
            crate::formats::zip_crypto::decrypt(keys, bytes);
        } else {
            let mut done = 0;
            while done < bytes.len() {
                let offset = self.position + done as u64;
                let counter = u128::from(offset / 16);
                let stream = Zeroizing::new(match &self.cipher {
                    Cipher::Aes(cipher) => cipher.stream_block(counter + 1),
                    Cipher::Lea(cipher) => cipher.block(counter.to_be_bytes()),
                    Cipher::Traditional(_) => unreachable!(),
                });
                let within = (offset % 16) as usize;
                let count = (16 - within).min(bytes.len() - done);
                for index in 0..count {
                    bytes[done + index] ^= stream[within + index];
                }
                done += count;
            }
        }
        self.position += bytes.len() as u64;
    }
    pub(super) fn reader<'a, R: Read>(
        &'a mut self,
        input: R,
        control: &'a OperationControl,
    ) -> Decrypted<'a, R> {
        Decrypted {
            input,
            state: self,
            control,
        }
    }
}
pub(super) struct Decrypted<'a, R> {
    input: R,
    state: &'a mut State,
    control: &'a OperationControl,
}
impl<R: Read> Read for Decrypted<'_, R> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.control.checkpoint().map_err(io::Error::other)?;
        let count = self.input.read(bytes)?;
        self.state.decrypt(&mut bytes[..count]);
        Ok(count)
    }
}

struct Lea {
    keys: Zeroizing<[[u32; 6]; 32]>,
    rounds: usize,
}
impl Lea {
    fn new(key: &[u8]) -> Self {
        const DELTA: [u32; 8] = [
            0xc3ef_e9db,
            0x4462_6b02,
            0x79e2_7c8a,
            0x78df_30ec,
            0x715e_a49e,
            0xc785_da0a,
            0xe04e_f22a,
            0xe5c4_0957,
        ];
        let words_count = key.len() / 4;
        let rounds = if words_count == 4 { 24 } else { 32 };
        let mut words = Zeroizing::new([0u32; 8]);
        for (index, chunk) in key.chunks_exact(4).enumerate() {
            words[index] = u32::from_le_bytes(chunk.try_into().expect("key word"));
        }
        let mut keys = Zeroizing::new([[0; 6]; 32]);
        for round in 0..rounds {
            let updated = if words_count == 4 { 4 } else { 6 };
            for (index, rotation) in [1, 3, 6, 11, 13, 17].iter().enumerate().take(updated) {
                let slot = if words_count == 4 {
                    index
                } else {
                    (6 * round + index) % 8
                };
                words[slot] = words[slot]
                    .wrapping_add(DELTA[round % words_count].rotate_left((round + index) as u32))
                    .rotate_left(*rotation);
            }
            for index in 0..6 {
                let slot = if words_count == 4 {
                    [0, 1, 2, 1, 3, 1][index]
                } else {
                    (6 * round + index) % 8
                };
                keys[round][index] = words[slot];
            }
        }
        Self { keys, rounds }
    }
    fn block(&self, block: [u8; 16]) -> [u8; 16] {
        let mut words = Zeroizing::new([0u32; 4]);
        for (index, chunk) in block.chunks_exact(4).enumerate() {
            words[index] = u32::from_le_bytes(chunk.try_into().expect("counter word"));
        }
        for key in self.keys.iter().take(self.rounds) {
            *words = [
                (words[0] ^ key[0])
                    .wrapping_add(words[1] ^ key[1])
                    .rotate_left(9),
                (words[1] ^ key[2])
                    .wrapping_add(words[2] ^ key[3])
                    .rotate_right(5),
                (words[2] ^ key[4])
                    .wrapping_add(words[3] ^ key[5])
                    .rotate_right(3),
                words[0],
            ];
        }
        let mut output = [0; 16];
        for (index, word) in words.iter().enumerate() {
            output[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        output
    }
}
