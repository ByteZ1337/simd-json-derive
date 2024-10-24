use simd_json_derive::Deserialize;

#[test]
fn opt() {
    #[derive(Deserialize, PartialEq, Debug)]
    #[serde(rename_all = "camelCase")]
    struct Rename {
        logo_name: Option<String>,
        #[serde(rename = "Name")]
        name: String,
    }
    let mut s = r#"{"Name": "snot", "logoName": "badger"}"#.to_string();
    let de =
        unsafe { Rename::from_str(s.as_mut_str()) }.expect("expected serialize with rename to work");
    assert_eq!(
        Rename {
            logo_name: Some("badger".to_string()),
            name: "snot".to_string()
        },
        de
    );
}