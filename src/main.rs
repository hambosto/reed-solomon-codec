use byteorder::{BigEndian, ByteOrder};
use reed_solomon_erasure::galois_8::ReedSolomon;
use std::io::{self, Error, ErrorKind};

const SHARD_LIMITS: ShardLimits = ShardLimits::new(1, 256);
const DATA_SIZE_LIMITS: DataSizeLimits = DataSizeLimits::new(1, 1 << 32);

#[derive(Debug, Clone, Copy)]
pub struct ShardLimits {
    pub min: usize,
    pub max: usize,
}

impl ShardLimits {
    const fn new(min: usize, max: usize) -> Self {
        Self { min, max }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DataSizeLimits {
    pub min: usize,
    pub max: usize,
}

impl DataSizeLimits {
    const fn new(min: usize, max: usize) -> Self {
        Self { min, max }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReedSolomonError {
    #[error("Invalid shard count: {0}")]
    InvalidShardCount(String),

    #[error("Invalid data size: {0}")]
    InvalidDataSize(String),

    #[error("Codec error: {0}")]
    CodecError(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Decoding error: {0}")]
    DecodingError(String),
}

impl From<ReedSolomonError> for io::Error {
    fn from(error: ReedSolomonError) -> Self {
        let kind = match error {
            ReedSolomonError::InvalidShardCount(_) | ReedSolomonError::InvalidDataSize(_) => {
                ErrorKind::InvalidInput
            }
            _ => ErrorKind::Other,
        };
        Error::new(kind, error.to_string())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EncoderConfig {
    data_shards: usize,
    parity_shards: usize,
    total_shards: usize,
}

impl EncoderConfig {
    pub fn new(data_shards: usize, parity_shards: usize) -> Result<Self, ReedSolomonError> {
        if !Self::is_valid_shard_count(data_shards) {
            return Err(ReedSolomonError::InvalidShardCount(format!(
                "Data shards must be between {} and {}",
                SHARD_LIMITS.min, SHARD_LIMITS.max
            )));
        }

        if !Self::is_valid_shard_count(parity_shards) {
            return Err(ReedSolomonError::InvalidShardCount(format!(
                "Parity shards must be between {} and {}",
                SHARD_LIMITS.min, SHARD_LIMITS.max
            )));
        }

        let total_shards = data_shards + parity_shards;
        if total_shards > SHARD_LIMITS.max {
            return Err(ReedSolomonError::InvalidShardCount(format!(
                "Total shards ({}) exceeds maximum allowed ({})",
                total_shards, SHARD_LIMITS.max
            )));
        }

        Ok(Self {
            data_shards,
            parity_shards,
            total_shards,
        })
    }

    fn is_valid_shard_count(count: usize) -> bool {
        (SHARD_LIMITS.min..=SHARD_LIMITS.max).contains(&count)
    }
}

#[derive(Debug)]
pub struct ReedSolomonCodec {
    codec: ReedSolomon,
    config: EncoderConfig,
}

impl ReedSolomonCodec {
    pub fn new(data_shards: usize, parity_shards: usize) -> Result<Self, ReedSolomonError> {
        let config: EncoderConfig = EncoderConfig::new(data_shards, parity_shards)?;

        let codec: reed_solomon_erasure::ReedSolomon<reed_solomon_erasure::galois_8::Field> =
            ReedSolomon::new(config.data_shards, config.parity_shards).map_err(
                |e: reed_solomon_erasure::Error| ReedSolomonError::CodecError(e.to_string()),
            )?;

        Ok(Self { codec, config })
    }

    pub fn encode(&self, data: &[u8]) -> Result<Vec<u8>, ReedSolomonError> {
        let encoded_data: Vec<u8> = DataProcessor::prepare_data(data)?;
        let mut shards: Vec<Vec<u8>> = DataProcessor::split_into_shards(
            &encoded_data,
            self.config.data_shards,
            self.config.total_shards,
        )?;

        let mut shard_refs: Vec<&mut [u8]> = shards
            .iter_mut()
            .map(|shard: &mut Vec<u8>| shard.as_mut_slice())
            .collect();

        self.codec
            .encode(&mut shard_refs)
            .map_err(|e: reed_solomon_erasure::Error| {
                ReedSolomonError::EncodingError(e.to_string())
            })?;

        Ok(shards.into_iter().flatten().collect())
    }

    pub fn decode(&self, data: &[u8]) -> Result<Vec<u8>, ReedSolomonError> {
        let shares: Vec<Vec<u8>> =
            DataProcessor::validate_and_split_shares(data, self.config.total_shards)?;
        let shard_size: usize = shares[0].len();

        let mut decode_buffer: Vec<u8> = vec![0u8; shard_size * self.config.data_shards];
        let mut decode_shards: Vec<_> = decode_buffer.chunks_mut(shard_size).collect();

        for (i, share) in shares.iter().take(self.config.data_shards).enumerate() {
            decode_shards[i].copy_from_slice(share);
        }

        DataProcessor::extract_original_data(&decode_buffer)
    }
}

struct DataProcessor;

impl DataProcessor {
    fn prepare_data(data: &[u8]) -> Result<Vec<u8>, ReedSolomonError> {
        Self::validate_data_size(data)?;

        let mut buffer: Vec<u8> = Vec::with_capacity(data.len() + 4);
        let mut size_prefix: [u8; 4] = [0u8; 4];
        BigEndian::write_u32(&mut size_prefix, data.len() as u32);

        buffer.extend_from_slice(&size_prefix);
        buffer.extend_from_slice(data);

        Ok(buffer)
    }

    fn validate_data_size(data: &[u8]) -> Result<(), ReedSolomonError> {
        if !(DATA_SIZE_LIMITS.min..=DATA_SIZE_LIMITS.max).contains(&data.len()) {
            return Err(ReedSolomonError::InvalidDataSize(format!(
                "Data size must be between {} and {}",
                DATA_SIZE_LIMITS.min, DATA_SIZE_LIMITS.max
            )));
        }
        Ok(())
    }

    fn split_into_shards(
        data: &[u8],
        data_shards: usize,
        total_shards: usize,
    ) -> Result<Vec<Vec<u8>>, ReedSolomonError> {
        let shard_size: usize = (data.len() + data_shards - 1) / data_shards;
        let mut shards: Vec<Vec<u8>> = vec![vec![0u8; shard_size]; total_shards];

        for (i, chunk) in data.chunks(shard_size).enumerate().take(data_shards) {
            shards[i][..chunk.len()].copy_from_slice(chunk);
        }

        Ok(shards)
    }

    fn validate_and_split_shares(
        data: &[u8],
        total_shards: usize,
    ) -> Result<Vec<Vec<u8>>, ReedSolomonError> {
        if data.is_empty() {
            return Err(ReedSolomonError::InvalidDataSize("Empty data".to_string()));
        }

        if data.len() % total_shards != 0 {
            return Err(ReedSolomonError::InvalidDataSize(format!(
                "Data length ({}) not divisible by total shards ({})",
                data.len(),
                total_shards
            )));
        }

        let share_size: usize = data.len() / total_shards;
        Ok((0..total_shards)
            .map(|i: usize| data[i * share_size..(i + 1) * share_size].to_vec())
            .collect())
    }

    fn extract_original_data(decoded: &[u8]) -> Result<Vec<u8>, ReedSolomonError> {
        if decoded.len() < 4 {
            return Err(ReedSolomonError::DecodingError(
                "Data too short".to_string(),
            ));
        }

        let original_size: usize = BigEndian::read_u32(&decoded[..4]) as usize;
        if original_size > decoded.len() - 4 {
            return Err(ReedSolomonError::DecodingError(
                "Invalid size prefix".to_string(),
            ));
        }

        Ok(decoded[4..4 + original_size].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() -> Result<(), ReedSolomonError> {
        let codec: ReedSolomonCodec = ReedSolomonCodec::new(10, 4)?;
        let original_data = b"Hello, World!".to_vec();

        let encoded: Vec<u8> = codec.encode(&original_data)?;
        let decoded: Vec<u8> = codec.decode(&encoded)?;

        assert_eq!(original_data, decoded);
        Ok(())
    }

    #[test]
    fn test_invalid_config() {
        assert!(ReedSolomonCodec::new(0, 1).is_err());
        assert!(ReedSolomonCodec::new(1, SHARD_LIMITS.max + 1).is_err());
        assert!(ReedSolomonCodec::new(SHARD_LIMITS.max / 2, SHARD_LIMITS.max / 2 + 1).is_err());
    }

    #[test]
    fn test_invalid_data_size() {
        let codec: ReedSolomonCodec = ReedSolomonCodec::new(10, 4).unwrap();
        let too_large: Vec<u8> = vec![0u8; DATA_SIZE_LIMITS.max + 1];
        assert!(codec.encode(&too_large).is_err());
    }
}

fn main() -> Result<(), ReedSolomonError> {
    let codec: ReedSolomonCodec = ReedSolomonCodec::new(10, 4)?;
    let original_data: Vec<u8> = b"Hello, World!".to_vec();

    let encoded: Vec<u8> = codec.encode(&original_data)?;
    let decoded: Vec<u8> = codec.decode(&encoded)?;

    println!("Original: {}", String::from_utf8_lossy(&original_data));
    println!("Encoded: {:?}", encoded);
    println!("Decoded: {}", String::from_utf8_lossy(&decoded));

    assert_eq!(original_data, decoded);
    Ok(())
}
