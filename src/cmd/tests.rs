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

#[test]
fn binary_keys_are_accepted() {
    let raw = Frame::array(vec![
        Frame::bulk(Bytes::from_static(b"SET")),
        Frame::bulk(Bytes::from_static(b"\xff\xfe\x00")),
        Frame::bulk(Bytes::from_static(b"v")),
    ]);
    match Command::from_frame(&raw).unwrap() {
        Command::Set { key, .. } => assert_eq!(&key[..], b"\xff\xfe\x00"),
        _ => panic!("expected SET"),
    }
}

#[test]
fn binary_set_members_and_hash_fields_are_accepted() {
    let sadd = Frame::array(vec![
        Frame::bulk(Bytes::from_static(b"SADD")),
        Frame::bulk(Bytes::from_static(b"s")),
        Frame::bulk(Bytes::from_static(b"\x80\x81")),
    ]);
    assert!(Command::from_frame(&sadd).is_ok());

    let hset = Frame::array(vec![
        Frame::bulk(Bytes::from_static(b"HSET")),
        Frame::bulk(Bytes::from_static(b"h")),
        Frame::bulk(Bytes::from_static(b"\xc3\x28")),
        Frame::bulk(Bytes::from_static(b"v")),
    ]);
    assert!(Command::from_frame(&hset).is_ok());
}
