use std::io::BufRead;
use std::io::{Seek, SeekFrom};

use anyhow::anyhow;

use crate::value::Value;

fn read_line<B: BufRead + Seek>(b: &mut B, position: u64) -> anyhow::Result<Option<String>> {
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

fn get_column(
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::value::Value;

    // --- get_column ---

    #[test]
    fn test_get_column_first() {
        let v = get_column("aaa;bbb;ccc", ';', 0, false).unwrap();
        assert_eq!(v, Value::Text("aaa".to_string()));
    }

    #[test]
    fn test_get_column_middle() {
        let v = get_column("aaa;bbb;ccc", ';', 1, false).unwrap();
        assert_eq!(v, Value::Text("bbb".to_string()));
    }

    #[test]
    fn test_get_column_last() {
        let v = get_column("aaa;bbb;ccc", ';', 2, false).unwrap();
        assert_eq!(v, Value::Text("ccc".to_string()));
    }

    #[test]
    fn test_get_column_last_with_newline() {
        // il \n deve essere ignorato anche sull'ultima colonna
        let v = get_column("aaa;bbb;ccc\n", ';', 2, false).unwrap();
        assert_eq!(v, Value::Text("ccc".to_string()));
    }

    #[test]
    fn test_get_column_numeric() {
        let v = get_column("42;foo;bar", ';', 0, true).unwrap();
        assert_eq!(v, Value::Number(42));
    }

    #[test]
    fn test_get_column_numeric_with_spaces() {
        // Value::build fa trim(), deve funzionare
        let v = get_column("  42  ;foo", ';', 0, true).unwrap();
        assert_eq!(v, Value::Number(42));
    }

    #[test]
    fn test_get_column_out_of_range() {
        // colonna 5 su una riga con 3 colonne → errore
        let result = get_column("aaa;bbb;ccc", ';', 5, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_column_invalid_numeric() {
        // colonna non numerica con flag -n → errore
        let result = get_column("aaa;bbb", ';', 0, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_column_custom_delimiter() {
        let v = get_column("aaa,bbb,ccc", ',', 1, false).unwrap();
        assert_eq!(v, Value::Text("bbb".to_string()));
    }

    #[test]
    fn test_get_column_empty_field() {
        // campo vuoto tra due delimitatori
        let v = get_column("aaa;;ccc", ';', 1, false).unwrap();
        assert_eq!(v, Value::Text("".to_string()));
    }

    // --- read_line ---

    #[test]
    fn test_read_line_from_start() {
        let mut c = Cursor::new(b"aaa\nbbb\nccc\n");
        let line = read_line(&mut c, 0).unwrap();
        assert_eq!(line, Some("aaa\n".to_string()));
    }

    #[test]
    fn test_read_line_from_mid_line() {
        // position=1 è a metà di "aaa" → deve skippare fino a \n e leggere "bbb"
        let mut c = Cursor::new(b"aaa\nbbb\nccc\n");
        let line = read_line(&mut c, 1).unwrap();
        assert_eq!(line, Some("bbb\n".to_string()));
    }

    #[test]
    fn test_read_line_exact_line_start() {
        // position=4 è esattamente l'inizio di "bbb"
        // ma position > 0 quindi skippa comunque "bbb\n" e legge "ccc"
        let mut c = Cursor::new(b"aaa\nbbb\nccc\n");
        let line = read_line(&mut c, 4).unwrap();
        assert_eq!(line, Some("ccc\n".to_string()));
    }

    #[test]
    fn test_read_line_last_line_with_newline() {
        // position=1 è dentro "aaa" → skippa "aaa\n" → legge "bbb\n"
        let mut c = Cursor::new(b"aaa\nbbb\n");
        let line = read_line(&mut c, 1).unwrap();
        assert_eq!(line, Some("bbb\n".to_string()));
    }

    #[test]
    fn test_read_line_last_line_without_newline() {
        // position=1 è dentro "aaa" → skippa "aaa\n" → legge "bbb" (senza \n finale)
        let mut c = Cursor::new(b"aaa\nbbb");
        let line = read_line(&mut c, 1).unwrap();
        assert_eq!(line, Some("bbb".to_string()));
    }

    #[test]
    fn test_read_line_single_line_with_newline() {
        let mut c = Cursor::new(b"aaa\n");
        let line = read_line(&mut c, 0).unwrap();
        assert_eq!(line, Some("aaa\n".to_string()));
    }

    #[test]
    fn test_read_line_single_line_without_newline() {
        let mut c = Cursor::new(b"aaa");
        let line = read_line(&mut c, 0).unwrap();
        assert_eq!(line, Some("aaa".to_string()));
    }

    #[test]
    fn test_read_line_empty_file() {
        let mut c = Cursor::new(b"");
        let line = read_line(&mut c, 0).unwrap();
        assert_eq!(line, None);
    }

    #[test]
    fn test_read_line_position_at_eof() {
        // position cade esattamente sull'ultimo \n → nessuna riga successiva
        let mut c = Cursor::new(b"aaa\n");
        let line = read_line(&mut c, 3).unwrap();
        assert_eq!(line, None);
    }

    #[test]
    fn test_read_line_position_beyond_eof() {
        let mut c = Cursor::new(b"aaa\n");
        let line = read_line(&mut c, 100).unwrap();
        assert_eq!(line, None);
    }

    // --- binary_search_in_file ---

    // Helper per costruire un Value testuale senza boilerplate
    fn txt(s: &str) -> Value {
        Value::Text(s.to_string())
    }
    fn num(n: i128) -> Value {
        Value::Number(n)
    }

    #[test]
    fn test_search_first_line() {
        // "aaa;1\n" inizia al byte 0
        let mut c = Cursor::new(b"aaa;1\nbbb;2\nccc;3\n");
        let pos = binary_search_in_file(&mut c, ';', &txt("aaa"), 0, false).unwrap();
        assert_eq!(pos, Some(0));
    }

    #[test]
    fn test_search_middle_line() {
        // "bbb;2\n" inizia al byte 6
        let mut c = Cursor::new(b"aaa;1\nbbb;2\nccc;3\n");
        let pos = binary_search_in_file(&mut c, ';', &txt("bbb"), 0, false).unwrap();
        assert_eq!(pos, Some(6));
    }

    #[test]
    fn test_search_last_line() {
        // "ccc;3\n" inizia al byte 12
        let mut c = Cursor::new(b"aaa;1\nbbb;2\nccc;3\n");
        let pos = binary_search_in_file(&mut c, ';', &txt("ccc"), 0, false).unwrap();
        assert_eq!(pos, Some(12));
    }

    #[test]
    fn test_search_not_found() {
        let mut c = Cursor::new(b"aaa;1\nbbb;2\nccc;3\n");
        let pos = binary_search_in_file(&mut c, ';', &txt("ddd"), 0, false).unwrap();
        assert_eq!(pos, None);
    }

    #[test]
    fn test_search_not_found_before_first() {
        // valore minore di tutti → None senza loop infinito
        let mut c = Cursor::new(b"bbb;1\nccc;2\nddd;3\n");
        let pos = binary_search_in_file(&mut c, ';', &txt("aaa"), 0, false).unwrap();
        assert_eq!(pos, None);
    }

    #[test]
    fn test_search_multiple_occurrences_returns_first() {
        // "aaa" compare due volte, deve restituire il byte 0 (prima occorrenza)
        let mut c = Cursor::new(b"aaa;1\naaa;2\nbbb;3\n");
        let pos = binary_search_in_file(&mut c, ';', &txt("aaa"), 0, false).unwrap();
        assert_eq!(pos, Some(0));
    }

    #[test]
    fn test_search_multiple_occurrences_in_middle() {
        // "bbb" compare due volte in mezzo al file
        // "aaa;1\n" = 6 byte → "bbb;1\n" inizia al byte 6
        let mut c = Cursor::new(b"aaa;1\nbbb;2\nbbb;3\nccc;4\n");
        let pos = binary_search_in_file(&mut c, ';', &txt("bbb"), 0, false).unwrap();
        assert_eq!(pos, Some(6));
    }

    #[test]
    fn test_search_empty_file() {
        let mut c = Cursor::new(b"");
        let pos = binary_search_in_file(&mut c, ';', &txt("aaa"), 0, false).unwrap();
        assert_eq!(pos, None);
    }

    #[test]
    fn test_search_single_line_found() {
        let mut c = Cursor::new(b"aaa;1\n");
        let pos = binary_search_in_file(&mut c, ';', &txt("aaa"), 0, false).unwrap();
        assert_eq!(pos, Some(0));
    }

    #[test]
    fn test_search_single_line_not_found() {
        let mut c = Cursor::new(b"aaa;1\n");
        let pos = binary_search_in_file(&mut c, ';', &txt("bbb"), 0, false).unwrap();
        assert_eq!(pos, None);
    }

    #[test]
    fn test_search_last_line_without_newline() {
        // file senza \n finale — "ccc;3" inizia al byte 12
        let mut c = Cursor::new(b"aaa;1\nbbb;2\nccc;3");
        let pos = binary_search_in_file(&mut c, ';', &txt("ccc"), 0, false).unwrap();
        assert_eq!(pos, Some(12));
    }

    #[test]
    fn test_search_numeric() {
        // "2;bar\n" inizia al byte 6
        let mut c = Cursor::new(b"1;foo\n2;bar\n3;baz\n");
        let pos = binary_search_in_file(&mut c, ';', &num(2), 0, true).unwrap();
        assert_eq!(pos, Some(6));
    }

    #[test]
    fn test_search_non_default_column() {
        // ricerca sulla colonna 1
        // "bbb;2\n" inizia al byte 6
        let mut c = Cursor::new(b"aaa;1\nbbb;2\nccc;3\n");
        let pos = binary_search_in_file(&mut c, ';', &num(2), 1, true).unwrap();
        assert_eq!(pos, Some(6));
    }
}
