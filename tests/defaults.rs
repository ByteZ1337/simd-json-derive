use simd_json_derive::Deserialize;

#[test]
fn default_in_named_struct() {
    #[derive(simd_json_derive::Deserialize, PartialEq, Debug)]
    struct Bla {
        #[serde(default)]
        f1: u8,
        f2: String,
    }

    let mut s = r#"{"f1":5,"f2":"badger"}"#.to_string();
    let b = Bla { f2: "badger".into(), f1: 5 };
    let b1 = unsafe { Bla::from_str(s.as_mut_str()) }.unwrap();
    assert_eq!(b, b1);

    let mut s = r#"{"f2":"badger"}"#.to_string();
    let b = Bla { f1: u8::default(), f2: "badger".into() };
    let b1 = unsafe { Bla::from_str(s.as_mut_str()) }.unwrap();
    assert_eq!(b, b1);

    let mut s = r#"{"f1":0}"#.to_string();
    let b1 = unsafe { Bla::from_str(s.as_mut_str()) };
    assert!(b1.is_err());
}