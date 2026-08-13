#[allow(dead_code)] // Applied as animated surfaces migrate to the shared layer.
pub(crate) const ENTER_DURATION_MS: u64 = 120;
#[allow(dead_code)] // Applied as animated surfaces migrate to the shared layer.
pub(crate) const EXIT_DURATION_MS: u64 = 120;
#[allow(dead_code)] // Applied as resizing surfaces migrate to the shared layer.
pub(crate) const RESIZE_DURATION_MS: u64 = 160;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_tokens_remain_short_and_bounded() {
        assert!((100..=140).contains(&ENTER_DURATION_MS));
        assert!((100..=140).contains(&EXIT_DURATION_MS));
        assert!((140..=180).contains(&RESIZE_DURATION_MS));
    }
}
