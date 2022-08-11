use std::collections::{BTreeMap, HashMap};
use std::io;
use std::io::prelude::*;

#[test]
fn test_buf() {
    let stdin = io::stdin();

    let mut stdin = stdin.lock();
    let len = {
        let buffer = stdin.fill_buf().unwrap();
        println!("{buffer:?}");
        buffer.len()
    };

    // ensure the bytes we worked with aren't returned again later.
    //    stdin.consume(len);
    println!("now len: {}", len);

    let new_len = {
        let buffer = stdin.fill_buf().unwrap();
        println!("{buffer:?}");
        buffer.len()
    };
    {
        println!("new len: {}", new_len);
        stdin.consume(new_len);
    }
    let buffer = stdin.fill_buf().unwrap();
    println!("{buffer:?}");
    println!("consume len: {}", buffer.len());
}

#[test]
fn state_get_data() {
    use regex::Regex;
    use std::fs::File;
    use std::io::BufReader;
    let re = Regex::new(r"(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})").unwrap();
    let buf = BufReader::new(File::open("data").unwrap());
    let mut counts = BTreeMap::new();
    for line in buf.lines() {
        let l = line.unwrap();
        for cap in re.captures_iter(l.as_str()) {
            let date = format!(
                "{}-{}-{}T{}:{}",
                &cap[1], &cap[2], &cap[3], &cap[4], &cap[5]
            );
            if counts.contains_key(date.as_str()) {
                *counts.get_mut(date.as_str()).unwrap() += 1;
            } else {
                counts.insert(date, 1);
            }
        }
    }

    println!("{:?}", counts);
    let sum: i32 = counts.values().into_iter().sum();
    println!("sum: {}", sum);
}
