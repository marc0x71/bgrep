use std::io::BufRead;
use std::io::{Seek, SeekFrom};

use anyhow::anyhow;

use crate::value::Value;

pub fn read_line<B: BufRead + Seek>(b: &mut B, position: u64) -> anyhow::Result<Option<String>> {
    let mut line = vec![];

    b.seek(SeekFrom::Start(position))?;
    if position > 0 {
        // If we're not at the beginning of the file, we're probably halfway down the line
        b.read_until(b'\n', &mut line)?;
        if line.is_empty() {
            // If we have reached EOF, there is no next line
            return Ok(None);
        }
        line.clear();
    }

    // Read the full line
    b.read_until(b'\n', &mut line)?;
    if line.is_empty() {
        // If we have reached EOF, there is no next line
        return Ok(None);
    }
    let s = String::from_utf8(line)?;

    Ok(Some(s))
}

pub fn get_column(
    line: &str,
    separator: char,
    position: usize,
    is_numeric: bool,
) -> anyhow::Result<Value> {
    let current = line
        .trim_end_matches(['\n'])
        .split(separator)
        .nth(position)
        .ok_or_else(|| anyhow!("Invalid line '{line}'"))?;
    Value::build(current, is_numeric)
}

pub fn binary_search_in_file<B: BufRead + Seek>(
    b: &mut B,
    separator: char,
    target: &Value,
    position: usize,
    is_numeric: bool,
) -> anyhow::Result<Option<u64>> {
    let file_size = b.seek(SeekFrom::End(0))?;
    if file_size == 0 {
        return Ok(None);
    }

    let mut left = 0;
    let mut right = file_size - 1;
    let mut result: Option<u64> = None;

    while left <= right {
        let mid = left + (right - left) / 2;
        // println!("<< left={left} mid={mid} right={right}");

        if let Some(line) = read_line(b, mid)? {
            // println!("line: {line}");

            let current = get_column(&line, separator, position, is_numeric)?;

            // println!("current: {:?} target:{:?}", current, target);

            if current == *target {
                result = Some(b.stream_position()? - (line.len() as u64));
                match mid.checked_sub(1) {
                    Some(r) => right = r,
                    None => break,
                }
            } else if current < *target {
                left = mid + 1;
            } else {
                match mid.checked_sub(1) {
                    Some(r) => right = r,
                    None => break,
                }
            }
        } else {
            match mid.checked_sub(1) {
                Some(r) => right = r,
                None => break,
            }
        }
        // println!(">> left={left} mid={mid} right={right}");
    }
    // println!("result={result:?}");

    Ok(result)
}

pub fn print_all_occurrences<B: BufRead + Seek>(
    b: &mut B,
    start_position: u64,
    separator: char,
    target: &Value,
    position: usize,
    is_numeric: bool,
) -> anyhow::Result<()> {
    b.seek(SeekFrom::Start(start_position))?;

    let mut s = String::new();

    loop {
        b.read_line(&mut s)?;
        if s.is_empty() {
            break;
        }
        let current = get_column(&s, separator, position, is_numeric)?;
        // dbg!(&current, &target);
        if current != *target {
            break;
        }
        print!("{s}");
        s.clear();
    }
    Ok(())
}
