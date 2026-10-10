use rand::RngCore;
use rand::SeedableRng;
use rand::rngs::SmallRng;

pub(crate) fn random_password() -> String {
    let mut generator = SmallRng::from_entropy();
    let mut entropy = [0u8; 24];
    generator.fill_bytes(&mut entropy);
    from_entropy(&entropy)
}

fn from_entropy(entropy: &[u8; 24]) -> String {
    const CHARACTERS: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+";
    let mut password = String::from("Aa1!");
    password.extend(
        entropy
            .iter()
            .map(|byte| char::from(CHARACTERS[usize::from(*byte) % CHARACTERS.len()])),
    );
    password
}

#[cfg(test)]
mod tests {
    use super::from_entropy;
    use super::random_password;

    fn assert_complexity(password: &str) {
        let categories = [
            password
                .chars()
                .any(|character| character.is_ascii_uppercase()),
            password
                .chars()
                .any(|character| character.is_ascii_lowercase()),
            password.chars().any(|character| character.is_ascii_digit()),
            password
                .chars()
                .any(|character| character.is_ascii_punctuation()),
        ];
        assert!(categories.into_iter().filter(|present| *present).count() >= 3);
        assert!(password.len() >= 24);
    }

    #[test]
    fn repeated_entropy_bytes_still_meet_windows_password_complexity() {
        for byte in 0..=u8::MAX {
            assert_complexity(&from_entropy(&[byte; 24]));
        }
    }

    #[test]
    fn passwords_keep_entropy_and_meet_complexity() {
        assert_ne!(from_entropy(&[0; 24]), from_entropy(&[255; 24]));
        assert_complexity(&random_password());
    }
}
