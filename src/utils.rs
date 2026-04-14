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
