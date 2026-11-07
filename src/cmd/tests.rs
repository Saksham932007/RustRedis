use super::*;

fn frame(parts: &[&str]) -> Frame {
    Frame::array(
        parts
            .iter()
            .map(|p| Frame::bulk(Bytes::copy_from_slice(p.as_bytes())))
            .collect(),
    )
}

fn parse(parts: &[&str]) -> Result<Command, String> {
    Command::from_frame(&frame(parts))
}

#[test]
fn set_with_ex_sets_an_expiry() {
    match parse(&["SET", "k", "v", "EX", "10"]).unwrap() {
        Command::Set { expires_at, .. } => assert!(expires_at.is_some()),
        _ => panic!("expected SET"),
    }
}

#[test]
fn set_ex_overflow_is_an_error_not_a_panic() {
    let err = parse(&["SET", "k", "v", "EX", "18446744073709551615"])
        .err()
        .unwrap();
    assert!(err.contains("invalid expire time"), "{}", err);
}

#[test]
fn set_ex_zero_is_rejected() {
    let err = parse(&["SET", "k", "v", "EX", "0"]).err().unwrap();
    assert!(err.contains("invalid expire time"), "{}", err);
}

#[test]
fn set_ex_requires_an_integer() {
    assert!(parse(&["SET", "k", "v", "EX", "abc"]).is_err());
    assert!(parse(&["SET", "k", "v", "EX", "-1"]).is_err());
}

#[test]
fn set_ex_without_a_value_is_a_syntax_error() {
    let err = parse(&["SET", "k", "v", "EX"]).err().unwrap();
    assert!(err.contains("syntax error"), "{}", err);
}
