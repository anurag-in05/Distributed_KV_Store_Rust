use std::io::{self, BufRead, Write};

use crate::kvstore::KvStore;

pub fn run(store: &mut KvStore) {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap() == 0 {
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match dispatch(store, line) {
            Some(response) => println!("{response}"),
            None => break,
        }
    }
}

fn dispatch(store: &mut KvStore, line: &str) -> Option<String> {
    let mut parts = line.split_whitespace();
    let command = parts.next().unwrap_or("").to_ascii_uppercase();

    let response = match command.as_str() {
        "SET" => match (parts.next(), parts.next(), parts.next()) {
            (Some(key), Some(value), None) => {
                store.set(key, value);
                "OK".to_string()
            }
            _ => "ERR wrong number of arguments for 'SET'".to_string(),
        },
        "GET" => match (parts.next(), parts.next()) {
            (Some(key), None) => match store.get(key) {
                Some(value) => value.clone(),
                None => "(nil)".to_string(),
            },
            _ => "ERR wrong number of arguments for 'GET'".to_string(),
        },
        "DEL" => match (parts.next(), parts.next()) {
            (Some(key), None) => bool_reply(store.del(key)),
            _ => "ERR wrong number of arguments for 'DEL'".to_string(),
        },
        "EXISTS" => match (parts.next(), parts.next()) {
            (Some(key), None) => bool_reply(store.exists(key)),
            _ => "ERR wrong number of arguments for 'EXISTS'".to_string(),
        },
        "PING" => "PONG".to_string(),
        "QUIT" => return None,
        other => format!("ERR unknown command '{other}'"),
    };

    Some(response)
}

fn bool_reply(value: bool) -> String {
    if value {
        "1".to_string()
    } else {
        "0".to_string()
    }
}
