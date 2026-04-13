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
