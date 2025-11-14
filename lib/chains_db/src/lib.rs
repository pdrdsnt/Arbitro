pub mod chains_db;
pub mod p_any;
pub mod p_config;
pub mod p_key;
pub mod p_state;
pub mod p_ticks;
pub mod p_tokens;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
