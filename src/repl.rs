use std::io::{self, BufRead, Write};
use std::time::Duration;

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
        "SET" => match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(key), Some(value), None, None) => {
                store.set(key, value, None);
                "OK".to_string()
            }
            (Some(key), Some(value), Some(ttl), None) => match ttl.parse::<u64>() {
                Ok(seconds) if seconds > 0 => {
                    store.set(key, value, Some(Duration::from_secs(seconds)));
                    "OK".to_string()
                }
                _ => "ERR invalid TTL".to_string(),
            },
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_replies_pong() {
        let mut store = KvStore::new();
        assert_eq!(dispatch(&mut store, "PING"), Some("PONG".to_string()));
    }

    #[test]
    fn quit_signals_none() {
        let mut store = KvStore::new();
        assert_eq!(dispatch(&mut store, "QUIT"), None);
    }

    #[test]
    fn unknown_command_is_reported() {
        let mut store = KvStore::new();
        assert_eq!(
            dispatch(&mut store, "NOPE"),
            Some("ERR unknown command 'NOPE'".to_string())
        );
    }

    #[test]
    fn set_get_del_exists_round_trip() {
        let mut store = KvStore::new();
        assert_eq!(dispatch(&mut store, "SET foo bar"), Some("OK".to_string()));
        assert_eq!(dispatch(&mut store, "GET foo"), Some("bar".to_string()));
        assert_eq!(dispatch(&mut store, "EXISTS foo"), Some("1".to_string()));
        assert_eq!(dispatch(&mut store, "DEL foo"), Some("1".to_string()));
        assert_eq!(dispatch(&mut store, "EXISTS foo"), Some("0".to_string()));
        assert_eq!(dispatch(&mut store, "GET foo"), Some("(nil)".to_string()));
    }

    #[test]
    fn set_wrong_arity_is_an_error() {
        let mut store = KvStore::new();
        assert_eq!(
            dispatch(&mut store, "SET foo"),
            Some("ERR wrong number of arguments for 'SET'".to_string())
        );
        assert_eq!(
            dispatch(&mut store, "SET foo bar 10 extra"),
            Some("ERR wrong number of arguments for 'SET'".to_string())
        );
    }

    #[test]
    fn set_rejects_non_positive_ttl() {
        let mut store = KvStore::new();
        assert_eq!(
            dispatch(&mut store, "SET foo bar 0"),
            Some("ERR invalid TTL".to_string())
        );
        assert_eq!(
            dispatch(&mut store, "SET foo bar notanumber"),
            Some("ERR invalid TTL".to_string())
        );
    }

    #[test]
    fn set_with_valid_ttl_is_accepted() {
        let mut store = KvStore::new();
        assert_eq!(
            dispatch(&mut store, "SET foo bar 30"),
            Some("OK".to_string())
        );
        assert_eq!(dispatch(&mut store, "GET foo"), Some("bar".to_string()));
    }

    #[test]
    fn command_names_are_case_insensitive() {
        let mut store = KvStore::new();
        assert_eq!(dispatch(&mut store, "ping"), Some("PONG".to_string()));
        assert_eq!(dispatch(&mut store, "set foo bar"), Some("OK".to_string()));
    }
}
