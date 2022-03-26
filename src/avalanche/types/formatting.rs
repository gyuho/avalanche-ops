// TODO: this is an invariant, so we should move to its own crate
// e.g., https://crates.io/crates/avalanche-format

use std::io::{self, Error, ErrorKind};

use bech32::{ToBase32, Variant};
use bitcoin::util::base58;

use crate::utils::{hash, vector};

const CHECKSUM_LENGTH: usize = 4;

/// Implements "formatting.EncodeWithChecksum" with "formatting.CB58".
/// "ids.ShortID.String" appends checksum to the digest bytes.
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/utils/formatting#EncodeWithChecksum
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/utils/hashing#Checksum
pub fn encode_cb58_with_checksum(d: &[u8]) -> String {
    // "hashing.Checksum" of "sha256.Sum256"
    let checksum = hash::compute_sha256(d);
    let checksum_length = checksum.len();
    let checksum = &checksum[checksum_length - CHECKSUM_LENGTH..];

    let mut checked = d.to_vec();
    let mut checksum = checksum.to_vec();
    checked.append(&mut checksum);

    // ref. "utils/formatting encode.CB58"
    // ref. "base58.Encode"
    base58::encode_slice(&checked)
}

/// Implements "formatting.Decode" with "formatting.CB58".
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/utils/formatting#Decode
pub fn decode_cb58_with_checksum(d: &str) -> io::Result<Vec<u8>> {
    let decoded = match base58::from(d) {
        Ok(v) => v,
        Err(e) => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("failed to decode base58 ({})", e),
            ));
        }
    };
    let decoded_length = decoded.len();

    // verify checksum
    let checksum = &decoded[decoded_length - CHECKSUM_LENGTH..];
    let orig = &decoded[..decoded_length - CHECKSUM_LENGTH];

    // "hashing.Checksum" of "sha256.Sum256"
    let orig_checksum = hash::compute_sha256(orig);
    let orig_checksum_length = orig_checksum.len();
    let orig_checksum = &orig_checksum[orig_checksum_length - CHECKSUM_LENGTH..];
    if !vector::eq_u8_vectors(checksum, orig_checksum) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("invalid checksum {:?} != {:?}", checksum, orig_checksum),
        ));
    }

    Ok(orig.to_vec())
}

/// Implements "formatting.Decode" with "formatting.Hex".
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/utils/formatting#Decode
pub fn decode_hex_with_checksum(d: &[u8]) -> io::Result<Vec<u8>> {
    let decoded = match hex::decode(d) {
        Ok(v) => v,
        Err(e) => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("failed to decode base58 ({})", e),
            ));
        }
    };
    let decoded_length = decoded.len();

    // verify checksum
    let checksum = &decoded[decoded_length - CHECKSUM_LENGTH..];
    let orig = &decoded[..decoded_length - CHECKSUM_LENGTH];

    // "hashing.Checksum" of "sha256.Sum256"
    let orig_checksum = hash::compute_sha256(orig);
    let orig_checksum_length = orig_checksum.len();
    let orig_checksum = &orig_checksum[orig_checksum_length - CHECKSUM_LENGTH..];
    if !vector::eq_u8_vectors(checksum, orig_checksum) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("invalid checksum {:?} != {:?}", checksum, orig_checksum),
        ));
    }

    Ok(orig.to_vec())
}

/// Implements "formatting.FormatAddress/FormatBech32".
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/utils/formatting#FormatAddress
/// ref. https://pkg.go.dev/github.com/ava-labs/avalanchego/utils/formatting#FormatBech32
pub fn address(chain_id_alias: &str, hrp: &str, d: &[u8]) -> io::Result<String> {
    assert_eq!(d.len(), 20);

    // No need to call "bech32.ConvertBits(payload, 8, 5, true)"
    // ".to_base32()" already does "bech32::convert_bits(d, 8, 5, true)"
    let encoded = match bech32::encode(hrp, d.to_base32(), Variant::Bech32) {
        Ok(enc) => enc,
        Err(e) => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("failed bech32::encode {}", e),
            ));
        }
    };
    Ok(format!("{}-{}", chain_id_alias, encoded))
}

#[test]
fn test_encode_c58_with_checksum() {
    // ref. https://github.com/ava-labs/avalanchego/blob/v1.7.5/utils/formatting/encoding_test.go#L71
    let d: Vec<u8> = Vec::new();
    let hashed = encode_cb58_with_checksum(&d);
    assert_eq!(hashed, "45PJLL");
    let decoded = decode_cb58_with_checksum(&hashed).unwrap();
    assert_eq!(d, decoded);

    let d: Vec<u8> = vec![0];
    let hashed = encode_cb58_with_checksum(&d);
    assert_eq!(hashed, "1c7hwa");
    let decoded = decode_cb58_with_checksum(&hashed).unwrap();
    assert_eq!(d, decoded);

    let d: Vec<u8> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 255];
    let hashed = encode_cb58_with_checksum(&d);
    assert_eq!(hashed, "1NVSVezva3bAtJesnUj");
    let decoded = decode_cb58_with_checksum(&hashed).unwrap();
    assert_eq!(d, decoded);

    let d: Vec<u8> = vec![
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
    ];
    let hashed = encode_cb58_with_checksum(&d);
    assert_eq!(hashed, "SkB92YpWm4Q2ijQHH34cqbKkCZWszsiQgHVjtNeFF2HdvDQU");
    let decoded = decode_cb58_with_checksum(&hashed).unwrap();
    assert_eq!(d, decoded);
}
