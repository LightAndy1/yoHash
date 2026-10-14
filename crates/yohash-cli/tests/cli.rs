use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;

fn cmd() -> Command {
    Command::cargo_bin("yohash").unwrap()
}

#[test]
fn single_file_prints_only_the_hash() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("a.txt");
    std::fs::File::create(&p)
        .unwrap()
        .write_all(b"abc")
        .unwrap();
    cmd().arg(&p).assert().success().stdout(predicate::eq(
        "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD\n",
    ));
}

#[test]
fn md5_lowercase_option() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("a.txt");
    std::fs::File::create(&p)
        .unwrap()
        .write_all(b"abc")
        .unwrap();
    cmd()
        .args(["-a", "md5", "-f", "lower"])
        .arg(&p)
        .assert()
        .success()
        .stdout(predicate::eq("900150983cd24fb0d6963f7d28e17f72\n"));
}

#[test]
fn multiple_files_print_name_colon_hash_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("hello world.txt");
    let b = dir.path().join("caf\u{e9}.txt");
    std::fs::File::create(&a)
        .unwrap()
        .write_all(b"abc")
        .unwrap();
    std::fs::File::create(&b)
        .unwrap()
        .write_all(b"abc")
        .unwrap();
    let out = cmd().arg(&a).arg(&b).output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0],
        format!(
            "{}: BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD",
            a.display()
        )
    );
    assert_eq!(
        lines[1],
        format!(
            "{}: BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD",
            b.display()
        )
    );
}

#[test]
fn missing_file_exits_one_with_message() {
    cmd()
        .arg("definitely-not-here-99999")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn one_bad_file_still_reports_the_good_one() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("good.txt");
    std::fs::File::create(&a)
        .unwrap()
        .write_all(b"abc")
        .unwrap();
    let out = cmd()
        .arg(&a)
        .arg("definitely-not-here-88888")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        text,
        format!(
            "{}: BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD\n",
            a.display()
        )
    );
}

#[test]
fn unknown_algorithm_exits_one() {
    cmd()
        .args(["-a", "blake3"])
        .arg("README.md")
        .assert()
        .failure()
        .code(1);
}
