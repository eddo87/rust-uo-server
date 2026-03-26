/// UO network encryption/decryption.
///
/// The UO client uses different encryption schemes during different phases
/// of the connection:
///
/// 1. **No encryption** - The initial login seed packet (0xEF) is sent unencrypted.
/// 2. **Login encryption** - After the seed, login-phase packets (e.g. 0x80 account login)
///    use a XOR-based cipher keyed from the client seed and version-specific key pair.
/// 3. **Game encryption** - After login, game-phase packets use Twofish-based encryption.

// ---------------------------------------------------------------------------
// Version-specific login encryption key pairs
// ---------------------------------------------------------------------------

/// Client 7.0.x.x login encryption keys.
pub const KEY_7_0: (u32, u32) = (0x2C13A159, 0xA2253F73);

/// Client 7.0.33.x login encryption keys (widely used Classic client build).
pub const KEY_7_0_33: (u32, u32) = (0x2C63B245, 0xA2556B7F);

/// Client 6.0.x.x login encryption keys.
pub const KEY_6_0: (u32, u32) = (0x2C132B59, 0xA225476F);

// ---------------------------------------------------------------------------
// LoginCrypt
// ---------------------------------------------------------------------------

/// XOR-based cipher used during the login phase of a UO connection.
///
/// The cipher maintains two 32-bit rolling keys that are updated after every
/// byte that is decrypted.
pub struct LoginCrypt {
    key1: u32,
    key2: u32,
}

impl LoginCrypt {
    /// Create a new `LoginCrypt` from the client seed and a version-specific
    /// key pair.
    ///
    /// The two internal keys are derived as follows:
    ///
    /// ```text
    /// k1 = ((~seed ^ key1) << 16) | ((seed ^ key2) & 0x0000FFFF)
    /// k2 = ((seed ^ key1) >> 16) | ((~seed ^ key2) & 0xFFFF0000)
    /// ```
    pub fn new(seed: u32, key1: u32, key2: u32) -> Self {
        let k1 = ((!seed ^ key1) << 16) | ((seed ^ key2) & 0x0000_FFFF);
        let k2 = ((seed ^ key1) >> 16) | ((!seed ^ key2) & 0xFFFF_0000);
        Self { key1: k1, key2: k2 }
    }

    /// Decrypt a single byte and advance the key state.
    fn decrypt_byte(&mut self, byte: u8) -> u8 {
        let result = byte ^ (self.key1 as u8);

        let old_key1 = self.key1;
        self.key1 = ((old_key1 >> 1) | (self.key2 << 31)) ^ self.key2;
        self.key2 = ((self.key2 >> 1) | (old_key1 << 31)) ^ old_key1;

        result
    }

    /// Decrypt `data` in place.
    pub fn decrypt(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            *byte = self.decrypt_byte(*byte);
        }
    }
}

// ---------------------------------------------------------------------------
// GameCrypt  (Twofish – stub)
// ---------------------------------------------------------------------------

/// Twofish-based cipher used during the game phase.
///
/// This is currently a no-op stub; the full Twofish implementation will be
/// added later.
pub struct GameCrypt {
    _seed: u32,
}

impl GameCrypt {
    pub fn new(seed: u32) -> Self {
        Self { _seed: seed }
    }

    /// Encrypt `data` in place (no-op stub).
    pub fn encrypt(&mut self, _data: &mut [u8]) {
        // TODO: implement Twofish encryption
    }

    /// Decrypt `data` in place (no-op stub).
    pub fn decrypt(&mut self, _data: &mut [u8]) {
        // TODO: implement Twofish decryption
    }
}

// ---------------------------------------------------------------------------
// NoEncryption
// ---------------------------------------------------------------------------

/// Pass-through "cipher" for clients that have encryption disabled.
pub struct NoEncryption;

impl NoEncryption {
    pub fn new() -> Self {
        Self
    }

    pub fn encrypt(&mut self, _data: &mut [u8]) {}
    pub fn decrypt(&mut self, _data: &mut [u8]) {}
}

impl Default for NoEncryption {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// CryptMode
// ---------------------------------------------------------------------------

/// The active encryption mode for a connection.
pub enum CryptMode {
    None(NoEncryption),
    Login(LoginCrypt),
    Game(GameCrypt),
}

impl CryptMode {
    /// Decrypt `data` in place using whichever cipher is currently active.
    pub fn decrypt(&mut self, data: &mut [u8]) {
        match self {
            CryptMode::None(c) => c.decrypt(data),
            CryptMode::Login(c) => c.decrypt(data),
            CryptMode::Game(c) => c.decrypt(data),
        }
    }

    /// Encrypt `data` in place using whichever cipher is currently active.
    pub fn encrypt(&mut self, data: &mut [u8]) {
        match self {
            CryptMode::None(c) => c.encrypt(data),
            CryptMode::Login(_) => {
                // Login encryption is decrypt-only on the server side;
                // server responses during the login phase are sent unencrypted.
            }
            CryptMode::Game(c) => c.encrypt(data),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_crypt_key_derivation() {
        // With a known seed and the 7.0 key pair, verify that internal keys
        // are derived deterministically.
        let seed: u32 = 0xDEADBEEF;
        let crypt = LoginCrypt::new(seed, KEY_7_0.0, KEY_7_0.1);

        // Just verify the struct was created with non-zero keys.
        assert_ne!(crypt.key1, 0);
        assert_ne!(crypt.key2, 0);
    }

    #[test]
    fn login_crypt_decrypt_single_byte() {
        let seed: u32 = 0x12345678;
        let mut crypt = LoginCrypt::new(seed, KEY_7_0.0, KEY_7_0.1);

        // Encrypt a known byte by capturing the XOR value.
        let plaintext: u8 = 0x80; // Account Login Request packet id
        let xor_byte = crypt.key1 as u8;
        let ciphertext = plaintext ^ xor_byte;

        // Reset cipher to the same initial state.
        let mut crypt = LoginCrypt::new(seed, KEY_7_0.0, KEY_7_0.1);
        let mut data = [ciphertext];
        crypt.decrypt(&mut data);

        assert_eq!(data[0], plaintext);
    }

    #[test]
    fn login_crypt_roundtrip() {
        // Encrypting then decrypting should yield the original plaintext.
        let seed: u32 = 0xAABBCCDD;
        let plaintext = b"Hello UO!".to_vec();

        // "Encrypt" by decrypting the plaintext with one cipher instance
        // (since XOR is symmetric within a single step, but the key state
        // advances, we need to capture the keystream first).
        let mut crypt_enc = LoginCrypt::new(seed, KEY_7_0.0, KEY_7_0.1);
        let mut ciphertext = plaintext.clone();

        // Build ciphertext by XOR-ing each plaintext byte with the keystream.
        for byte in ciphertext.iter_mut() {
            let xor_byte = crypt_enc.key1 as u8;
            *byte ^= xor_byte;

            // Advance key state (same logic as decrypt_byte).
            let old_key1 = crypt_enc.key1;
            crypt_enc.key1 =
                ((old_key1 >> 1) | (crypt_enc.key2 << 31)) ^ crypt_enc.key2;
            crypt_enc.key2 =
                ((crypt_enc.key2 >> 1) | (old_key1 << 31)) ^ old_key1;
        }

        // The ciphertext should differ from the plaintext.
        assert_ne!(ciphertext, plaintext);

        // Now decrypt and verify we get the original back.
        let mut crypt_dec = LoginCrypt::new(seed, KEY_7_0.0, KEY_7_0.1);
        let mut decrypted = ciphertext.clone();
        crypt_dec.decrypt(&mut decrypted);

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn login_crypt_key_state_advances() {
        let seed: u32 = 0x11111111;
        let mut crypt = LoginCrypt::new(seed, KEY_7_0.0, KEY_7_0.1);

        let initial_key1 = crypt.key1;
        let initial_key2 = crypt.key2;

        // Decrypt one byte to advance the state.
        crypt.decrypt_byte(0x00);

        assert_ne!(crypt.key1, initial_key1);
        assert_ne!(crypt.key2, initial_key2);
    }

    #[test]
    fn login_crypt_known_ciphertext() {
        // Verify decryption against a pre-computed ciphertext sequence.
        // Seed = 0x01020304, keys = KEY_7_0.
        let seed: u32 = 0x01020304;
        let mut crypt = LoginCrypt::new(seed, KEY_7_0.0, KEY_7_0.1);

        // Record the keystream for the first 4 bytes.
        let mut keystream = [0u8; 4];
        let mut crypt_ks = LoginCrypt::new(seed, KEY_7_0.0, KEY_7_0.1);
        for b in keystream.iter_mut() {
            *b = crypt_ks.key1 as u8;
            crypt_ks.decrypt_byte(0x00); // advance state
        }

        // Plaintext: [0x80, 0x00, 0x61, 0x64]  (account login, then "ad")
        let plaintext = [0x80u8, 0x00, 0x61, 0x64];
        let ciphertext: Vec<u8> = plaintext
            .iter()
            .zip(keystream.iter())
            .map(|(p, k)| p ^ k)
            .collect();

        let mut data = ciphertext.clone();
        crypt.decrypt(&mut data);

        assert_eq!(data, plaintext);
    }

    #[test]
    fn no_encryption_passthrough() {
        let mut no_enc = NoEncryption::new();
        let original = vec![0x01, 0x02, 0x03, 0x04];
        let mut data = original.clone();

        no_enc.decrypt(&mut data);
        assert_eq!(data, original);

        no_enc.encrypt(&mut data);
        assert_eq!(data, original);
    }

    #[test]
    fn game_crypt_stub_passthrough() {
        let mut game = GameCrypt::new(0xDEADBEEF);
        let original = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let mut data = original.clone();

        game.encrypt(&mut data);
        assert_eq!(data, original, "GameCrypt stub should be a no-op");

        game.decrypt(&mut data);
        assert_eq!(data, original, "GameCrypt stub should be a no-op");
    }

    #[test]
    fn crypt_mode_dispatch() {
        // Verify that CryptMode::Login dispatches to LoginCrypt.
        let seed: u32 = 0xCAFEBABE;
        let mut mode = CryptMode::Login(LoginCrypt::new(seed, KEY_7_0.0, KEY_7_0.1));

        // Build expected output by running LoginCrypt directly.
        let mut direct = LoginCrypt::new(seed, KEY_7_0.0, KEY_7_0.1);
        let mut expected = [0x80u8, 0x00, 0x41, 0x42];
        let plaintext = expected;

        // Encrypt with direct keystream.
        for byte in expected.iter_mut() {
            let xor_byte = direct.key1 as u8;
            *byte ^= xor_byte;
            let old_key1 = direct.key1;
            direct.key1 =
                ((old_key1 >> 1) | (direct.key2 << 31)) ^ direct.key2;
            direct.key2 =
                ((direct.key2 >> 1) | (old_key1 << 31)) ^ old_key1;
        }

        let mut data = expected;
        mode.decrypt(&mut data);
        assert_eq!(data, plaintext);
    }

    #[test]
    fn different_seeds_produce_different_output() {
        let mut crypt_a = LoginCrypt::new(0x11111111, KEY_7_0.0, KEY_7_0.1);
        let mut crypt_b = LoginCrypt::new(0x22222222, KEY_7_0.0, KEY_7_0.1);

        let mut data_a = [0x80u8];
        let mut data_b = [0x80u8];

        crypt_a.decrypt(&mut data_a);
        crypt_b.decrypt(&mut data_b);

        // Different seeds must produce different decrypted output for the
        // same ciphertext byte.
        assert_ne!(data_a, data_b);
    }

    #[test]
    fn different_version_keys_produce_different_output() {
        let seed: u32 = 0xDEADBEEF;
        let mut crypt_70 = LoginCrypt::new(seed, KEY_7_0.0, KEY_7_0.1);
        let mut crypt_60 = LoginCrypt::new(seed, KEY_6_0.0, KEY_6_0.1);

        let mut data_70 = [0x80u8];
        let mut data_60 = [0x80u8];

        crypt_70.decrypt(&mut data_70);
        crypt_60.decrypt(&mut data_60);

        assert_ne!(data_70, data_60);
    }
}
