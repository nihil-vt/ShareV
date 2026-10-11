use std::sync::{Mutex, OnceLock};

/// Monotonic ULID source shared across the app.
fn generator() -> &'static Mutex<ulid::Generator> {
    static GENERATOR: OnceLock<Mutex<ulid::Generator>> = OnceLock::new();
    GENERATOR.get_or_init(|| Mutex::new(ulid::Generator::new()))
}

/// Generate a new, lexicographically sortable capture id.
pub fn new_id() -> String {
    let mut generator = generator().lock().unwrap();
    match generator.generate() {
        Ok(id) => id.to_string(),
        // Only happens if the random bits overflow within the same millisecond.
        Err(overflow) => overflow.commit_overflow_increment().to_string(),
    }
}
