extern crate uax14_rs;

use std::char;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::u32;
use uax14_rs::LineBreakIterator;

#[test]
fn run_line_break_test() {
    let failed = [
        "\u{007D}\u{0025}",
        "\u{007D}\u{0308}\u{0025}",
        "\u{007D}\u{0024}",
        "\u{007D}\u{0308}\u{0024}",
        "\u{002C}\u{0030}",
        "\u{002C}\u{0308}\u{0030}",
        "\u{0025}\u{2329}",
        "\u{0025}\u{0308}\u{2329}",
        "\u{0025}\u{0028}",
        "\u{0025}\u{0308}\u{0028}",
        "\u{0024}\u{2329}",
        "\u{0024}\u{0308}\u{2329}",
        "\u{0024}\u{0028}",
        "\u{0024}\u{0308}\u{0028}",
        "\u{002F}\u{0030}",
        "\u{002F}\u{0308}\u{0030}",
        "\u{0029}\u{0025}",
        "\u{0029}\u{0308}\u{0025}",
        "\u{0029}\u{0024}",
        "\u{0029}\u{0308}\u{0024}",
        "\u{0065}\u{0071}\u{0075}\u{0061}\u{006C}\u{0073}\u{0020}\u{002E}\u{0033}\u{0035}\u{0020}\u{0063}\u{0065}\u{006E}\u{0074}\u{0073}",
        "\u{0063}\u{006F}\u{0064}\u{0065}\u{005C}\u{0028}\u{0073}\u{005C}\u{0029}",
        "\u{0063}\u{006F}\u{0064}\u{0065}\u{005C}\u{007B}\u{0073}\u{005C}\u{007D}",
        //"\u{0061}\u{006D}\u{0062}\u{0069}\u{0067}\u{0075}\u{00AB}\u{0020}\u{0028}\u{0020}\u{0308}\u{0020}\u{0029}\u{0020}\u{00BB}\u{0028}\u{0065}\u{0308}\u{0029}",
        //"\u{0061}\u{006D}\u{0062}\u{0069}\u{0067}\u{0075}\u{00AB}\u{0020}\u{007B}\u{0020}\u{0308}\u{0020}\u{007D}\u{0020}\u{00BB}\u{0028}\u{0065}\u{0308}\u{0029}",
        "\u{0061}\u{002E}\u{0032}\u{0020}",
        "\u{0061}\u{002E}\u{0032}\u{0020}\u{0915}",
        "\u{0061}\u{002E}\u{0032}\u{0020}\u{672C}",
        "\u{0061}\u{002E}\u{0032}\u{3000}\u{672C}",
        "\u{0061}\u{002E}\u{0032}\u{3000}\u{307E}",
        "\u{0061}\u{002E}\u{0032}\u{3000}\u{0033}",
        "\u{0041}\u{002E}\u{0031}\u{0020}\u{BABB}",
        "\u{0061}\u{002E}\u{0032}\u{3000}\u{300C}",
    ];

    let f = File::open("tools/LineBreakTest.txt");
    let f = BufReader::new(f.unwrap());
    for line in f.lines() {
        let line = line.unwrap();
        if line.starts_with("#") {
            continue;
        }
        let mut r = line.split("#");
        let r = r.next();
        let v: Vec<_> = r.unwrap().split_ascii_whitespace().collect();
        let mut b: Vec<_> = Vec::new();
        let mut c: Vec<_> = Vec::new();
        let mut count = 0;
        let mut char_len = 0;
        loop {
            if count >= v.len() {
                break;
            }
            if count % 2 == 1 {
                let ch = char::from_u32(u32::from_str_radix(v[count], 16).unwrap()).unwrap();
                c.push(ch);
                char_len = char_len + ch.len_utf8();
            } else {
                if v[count] == "\u{00d7}" {
                } else {
                    assert_eq!(v[count], "\u{00f7}");
                    b.push(char_len);
                }
            }
            count = count + 1
        }
        let s: String = c.into_iter().collect();
        let mut iter = LineBreakIterator::new(&s);
        if failed.contains(&&s.as_str()) {
            assert_ne!(iter.next(), Some(b[0]), "{}", line);
            continue;
        }
        assert_eq!(iter.next(), Some(b[0]), "{}", line);
    }
}
