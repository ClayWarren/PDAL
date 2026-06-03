//! Streaming zlib (DEFLATE) compression, ported from `pdal/compression/`.
//!
//! Mirrors the streaming `DeflateCompressor` / `DeflateDecompressor` helpers in
//! `pdal/compression/DeflateCompression.cpp`. The regular helpers produce and
//! consume the zlib wire format (zlib header + Adler-32 trailer), matching
//! zlib's `deflateInit`/`inflateInit2(15)` defaults. The auto decompressor
//! mirrors `inflateInit2(47)` by accepting either zlib or gzip input.

use flate2::write::{MultiGzDecoder, ZlibDecoder, ZlibEncoder};
use flate2::Compression;
use std::io::Write;
use std::mem;

/// Incremental zlib compressor.
///
/// Feed input with [`DeflateCompressor::update`] and flush the stream with
/// [`DeflateCompressor::finish`]. Each call returns the compressed bytes
/// produced so far, which may be empty until enough input has accumulated.
pub struct DeflateCompressor {
    encoder: Option<ZlibEncoder<Vec<u8>>>,
}

impl DeflateCompressor {
    pub fn new() -> Self {
        DeflateCompressor {
            encoder: Some(ZlibEncoder::new(Vec::new(), Compression::default())),
        }
    }

    /// Compress `input`, returning whatever compressed bytes are now available.
    pub fn update(&mut self, input: &[u8]) -> Result<Vec<u8>, String> {
        let encoder = self
            .encoder
            .as_mut()
            .ok_or("deflate compressor already finished")?;
        encoder.write_all(input).map_err(|err| err.to_string())?;
        Ok(mem::take(encoder.get_mut()))
    }

    /// Flush and finalize the stream, returning the remaining compressed bytes.
    pub fn finish(&mut self) -> Result<Vec<u8>, String> {
        let encoder = self
            .encoder
            .take()
            .ok_or("deflate compressor already finished")?;
        encoder.finish().map_err(|err| err.to_string())
    }
}

impl Default for DeflateCompressor {
    fn default() -> Self {
        Self::new()
    }
}

/// Incremental zlib decompressor.
pub struct DeflateDecompressor {
    decoder: Option<ZlibDecoder<Vec<u8>>>,
}

impl DeflateDecompressor {
    pub fn new() -> Self {
        DeflateDecompressor {
            decoder: Some(ZlibDecoder::new(Vec::new())),
        }
    }

    /// Decompress `input`, returning whatever decoded bytes are now available.
    pub fn update(&mut self, input: &[u8]) -> Result<Vec<u8>, String> {
        let decoder = self
            .decoder
            .as_mut()
            .ok_or("deflate decompressor already finished")?;
        decoder.write_all(input).map_err(|err| err.to_string())?;
        Ok(mem::take(decoder.get_mut()))
    }

    /// Finalize the stream, returning the remaining decoded bytes.
    pub fn finish(&mut self) -> Result<Vec<u8>, String> {
        let decoder = self
            .decoder
            .take()
            .ok_or("deflate decompressor already finished")?;
        decoder.finish().map_err(|err| err.to_string())
    }
}

impl Default for DeflateDecompressor {
    fn default() -> Self {
        Self::new()
    }
}

enum AutoDecoder {
    Undecided(Vec<u8>),
    Zlib(ZlibDecoder<Vec<u8>>),
    Gzip(MultiGzDecoder<Vec<u8>>),
    Finished,
}

/// Incremental decompressor that accepts zlib or gzip input.
pub struct AutoDecompressor {
    decoder: AutoDecoder,
}

impl AutoDecompressor {
    pub fn new() -> Self {
        Self {
            decoder: AutoDecoder::Undecided(Vec::new()),
        }
    }

    pub fn update(&mut self, input: &[u8]) -> Result<Vec<u8>, String> {
        match &mut self.decoder {
            AutoDecoder::Undecided(pending) => {
                pending.extend_from_slice(input);
                if pending.len() < 2 {
                    return Ok(Vec::new());
                }
                let pending = mem::take(pending);
                let mut next = if pending.starts_with(&[0x1f, 0x8b]) {
                    AutoDecoder::Gzip(MultiGzDecoder::new(Vec::new()))
                } else {
                    AutoDecoder::Zlib(ZlibDecoder::new(Vec::new()))
                };
                let out = write_auto_decoder(&mut next, &pending)?;
                self.decoder = next;
                Ok(out)
            }
            AutoDecoder::Zlib(_) | AutoDecoder::Gzip(_) => {
                write_auto_decoder(&mut self.decoder, input)
            }
            AutoDecoder::Finished => Err("auto decompressor already finished".to_string()),
        }
    }

    pub fn finish(&mut self) -> Result<Vec<u8>, String> {
        let decoder = mem::replace(&mut self.decoder, AutoDecoder::Finished);
        match decoder {
            AutoDecoder::Undecided(pending) if pending.is_empty() => Ok(Vec::new()),
            AutoDecoder::Undecided(pending) => {
                let mut decoder = ZlibDecoder::new(Vec::new());
                decoder.write_all(&pending).map_err(|err| err.to_string())?;
                decoder.finish().map_err(|err| err.to_string())
            }
            AutoDecoder::Zlib(decoder) => decoder.finish().map_err(|err| err.to_string()),
            AutoDecoder::Gzip(decoder) => decoder.finish().map_err(|err| err.to_string()),
            AutoDecoder::Finished => Err("auto decompressor already finished".to_string()),
        }
    }
}

impl Default for AutoDecompressor {
    fn default() -> Self {
        Self::new()
    }
}

fn write_auto_decoder(decoder: &mut AutoDecoder, input: &[u8]) -> Result<Vec<u8>, String> {
    match decoder {
        AutoDecoder::Zlib(decoder) => {
            decoder.write_all(input).map_err(|err| err.to_string())?;
            Ok(mem::take(decoder.get_mut()))
        }
        AutoDecoder::Gzip(decoder) => {
            decoder.write_all(input).map_err(|err| err.to_string())?;
            Ok(mem::take(decoder.get_mut()))
        }
        AutoDecoder::Undecided(_) => Ok(Vec::new()),
        AutoDecoder::Finished => Err("auto decompressor already finished".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(data: &[u8]) -> Vec<u8> {
        let mut compressor = DeflateCompressor::new();
        let mut compressed = compressor.update(data).unwrap();
        compressed.extend(compressor.finish().unwrap());

        let mut decompressor = DeflateDecompressor::new();
        let mut decoded = decompressor.update(&compressed).unwrap();
        decoded.extend(decompressor.finish().unwrap());
        decoded
    }

    #[test]
    fn round_trips_empty_input() {
        assert_eq!(round_trip(&[]), Vec::<u8>::new());
    }

    #[test]
    fn round_trips_small_input() {
        let data = b"the quick brown fox jumps over the lazy dog";
        assert_eq!(round_trip(data), data);
    }

    #[test]
    fn round_trips_large_input_across_multiple_updates() {
        let data: Vec<u8> = (0..200_000u32)
            .map(|i| i.wrapping_mul(2654435761) as u8)
            .collect();

        let mut compressor = DeflateCompressor::new();
        let mut compressed = Vec::new();
        for chunk in data.chunks(7919) {
            compressed.extend(compressor.update(chunk).unwrap());
        }
        compressed.extend(compressor.finish().unwrap());

        let mut decompressor = DeflateDecompressor::new();
        let mut decoded = Vec::new();
        for chunk in compressed.chunks(4096) {
            decoded.extend(decompressor.update(chunk).unwrap());
        }
        decoded.extend(decompressor.finish().unwrap());
        assert_eq!(decoded, data);
    }

    #[test]
    fn compresses_repetitive_data() {
        let data = vec![7u8; 100_000];
        let mut compressor = DeflateCompressor::new();
        let mut compressed = compressor.update(&data).unwrap();
        compressed.extend(compressor.finish().unwrap());
        assert!(compressed.len() < data.len());
        assert_eq!(round_trip(&data), data);
    }

    #[test]
    fn update_after_finish_is_an_error() {
        let mut compressor = DeflateCompressor::new();
        compressor.finish().unwrap();
        assert!(compressor.update(b"more").is_err());

        let mut decompressor = DeflateDecompressor::new();
        decompressor.finish().unwrap();
        assert!(decompressor.update(b"more").is_err());
    }

    #[test]
    fn decompressing_garbage_is_an_error() {
        let mut decompressor = DeflateDecompressor::new();
        let err = decompressor.update(&[0xde, 0xad, 0xbe, 0xef, 0x00, 0x11]);
        assert!(err.is_err() || decompressor.finish().is_err());
    }

    #[test]
    fn auto_decompressor_accepts_zlib_stream() {
        let data = b"zlib payload";
        let mut compressor = DeflateCompressor::new();
        let mut compressed = compressor.update(data).unwrap();
        compressed.extend(compressor.finish().unwrap());

        let mut decompressor = AutoDecompressor::new();
        let mut decoded = Vec::new();
        for chunk in compressed.chunks(1) {
            decoded.extend(decompressor.update(chunk).unwrap());
        }
        decoded.extend(decompressor.finish().unwrap());
        assert_eq!(decoded, data);
    }

    #[test]
    fn auto_decompressor_accepts_gzip_stream() {
        use flate2::write::GzEncoder;

        let data = b"gzip payload";
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut decompressor = AutoDecompressor::new();
        let mut decoded = Vec::new();
        for chunk in compressed.chunks(3) {
            decoded.extend(decompressor.update(chunk).unwrap());
        }
        decoded.extend(decompressor.finish().unwrap());
        assert_eq!(decoded, data);
    }
}
