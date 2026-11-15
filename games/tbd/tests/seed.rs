//! Decimal strings are engine seeds. Other strings are a stable FNV-1a.

use tbd::{seed_from_f64, seed_from_text, Config, TbdGame};

#[test]
fn decimal_strings_match_the_integer_and_other_text_is_stable() {
    assert_eq!(seed_from_text("42"), 42);
    assert_eq!(seed_from_text(" 42 "), 42);
    assert_eq!(seed_from_text("18446744073709551615"), u64::MAX);
    assert_eq!(seed_from_f64(42.0).unwrap(), 42);
    assert!(seed_from_f64(1.5).is_err());
    assert!(seed_from_f64(-1.0).is_err());
    let a = seed_from_text("north-pad");
    let b = seed_from_text("north-pad");
    let c = seed_from_text("south-pad");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(a, 42);
    let cfg = Config {
        wind: true,
        ..Config::default()
    };
    let mut from_text = TbdGame::new(a, cfg.clone()).unwrap();
    let mut again = TbdGame::new(seed_from_text("north-pad"), cfg).unwrap();
    let mut obs = [0.0; tbd::OBS_LEN];
    obs[6] = 1.0;
    for _ in 0..5 {
        assert_eq!(from_text.step(&obs).unwrap(), again.step(&obs).unwrap());
    }
}
