use std::sync::{Mutex, OnceLock};

static RNG_STATE: OnceLock<Mutex<u64>> = OnceLock::new();

fn get_rng_state() -> &'static Mutex<u64> {
    RNG_STATE.get_or_init(|| Mutex::new(1))
}

pub fn get_env(key: &str) -> Option<String> {
    if key.is_empty() || key.contains('=') || key.contains('\0') {
        return None;
    }
    std::env::var(key).ok()
}

pub fn set_env(key: &str, value: &str) -> i32 {
    if key.is_empty() || key.contains('=') || key.contains('\0') || value.contains('\0') {
        return -1;
    }
    std::env::set_var(key, value);
    0
}

pub fn unset_env(key: &str) -> i32 {
    if key.is_empty() || key.contains('=') || key.contains('\0') {
        return -1;
    }
    std::env::remove_var(key);
    0
}

pub fn random_seed(seed: u32) {
    let mut state = get_rng_state().lock().unwrap();
    *state = seed as u64;
}

pub fn random(minimum: f64, maximum: f64) -> f64 {
    let mut state = get_rng_state().lock().unwrap();
    let next = state.wrapping_mul(1103515245).wrapping_add(12345);
    *state = next;

    let r = next as f64 / u64::MAX as f64;
    let val = minimum + r * (maximum - minimum);
    val.clamp(minimum, maximum)
}
