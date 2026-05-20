pub use anyhow::{bail, Context};
pub use lazy_static::lazy_static;

use std::sync::{Mutex, MutexGuard};
pub type AnyErr = anyhow::Error;

pub fn strip_www(s: &String) -> String {
    if s.len() >= 4 {
        if &s[0..=3] == "www." {
            String::from(&s[4..=s.len() - 1])
        } else {
            s.clone()
        }
    } else {
        s.clone()
    }
}

// pub fn strip_leading_slash(s: String) -> String {
//     if s.len() >= 1 {
//         if &s[0] == '/' {
//             String::from(&s[1..d.len() - 1])
//         } else {
//             s
//         }
//     } else {
//         s.clone()
//     }
// }

//index out of bounds if string.len() < n
pub fn last_n(s: &String, n: usize) -> &str {
    &s[s.len() - n..=s.len() - 1]
}

//index out of bounds if string.len() < n
pub fn last_n_mut(s: &mut String, n: usize) -> &mut str {
    let len = s.len();
    &mut s[len - n..=len - 1]
}

pub fn get_ext(s: &String) -> Option<&str> {
    let parts = s.split("/");
    let last = parts.last()?;
    if last.len() < 3 {
        return None;
    } else {
        let idx = last.find(".")?;
        return Some(last_n(s, s.len() - idx));
    }
}

lazy_static! {
    static ref GOV_LOCK_RATE: std::time::Duration = std::time::Duration::from_millis(20); // miliseconds
}

pub fn open_mutex<T>(lock: &Mutex<T>) -> MutexGuard<T> {
    use std::sync::TryLockError::*;
    'ACCESS: loop {
        match lock.try_lock() {
            Ok(data) => {
                return data;
            }
            Err(e) => match e {
                WouldBlock => {
                    tokio::time::sleep(*GOV_LOCK_RATE);
                }
                PoisonError => {
                    panic!("mutex is poisoned");
                }
            },
        }
    }
}
