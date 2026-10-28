use sha2::{Digest, Sha256};

pub struct Nonce {
    pub raw: String,
    pub hash: String,
}

pub fn generate() -> Result<Nonce, String> {
    let mut bytes = [0_u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|_| "Could not generate nonce".to_string())?;
    Ok(from_bytes(bytes))
}

fn from_bytes(bytes: [u8; 32]) -> Nonce {
    let raw = hex::encode(bytes);
    let hash = hex::encode(Sha256::digest(raw.as_bytes()));
    Nonce { raw, hash }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_hash_matches_raw_nonce_text() {
        let nonce = from_bytes([0; 32]);
        assert_eq!(nonce.raw, "0".repeat(64));
        assert_eq!(
            nonce.hash,
            "60e05bd1b195af2f94112fa7197a5c88289058840ce7c6df9693756bc6250f55"
        );
    }
}
