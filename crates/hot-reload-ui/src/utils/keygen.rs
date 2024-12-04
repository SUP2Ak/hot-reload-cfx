use rand::{thread_rng, Rng};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

pub fn generate_api_key() -> String {
    let mut rng = thread_rng();
    let mut bytes = vec![0u8; 32];
    rng.fill(&mut bytes[..]);
    BASE64.encode(&bytes)
}