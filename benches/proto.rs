#[bench]
fn sync_log_with_meta1(b: &mut Bencher) {
    let buf = r#"{"timestamp": 1.469183191579457e+18, "message": "le message", "pid": 93111, "severity": [0, "DEBUG"], "thread": 2}"#;
    let nread =

    b.iter(|| {
        serde_json::from_slice::<Value>(&buf[..nread])
    });
}
