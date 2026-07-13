use crate::utils::*;
use url::Url;

// //todo: I know there is a lot of going back and forth between strings and Url's thats not strictly necessary.
// fn normalize_url(url: &String, root: Option<reqwest::Url>) -> Result<String, AnyErr> {
//     match Url::parse(url) {
//         Err(e) => {
//             if url.starts_with("/") {
//                 match root {
//                     Some(root_contents) => {
//                         let joined = root_contents.join(url)?;
//                         normalize_url(&String::from(joined), None)
//                     }
//                     None => Err(From::from(e)),
//                 }
//             } else {
//                 return Err(From::from(e));
//             }
//         }
//         Ok(parsed_url) => Ok(String::from(parsed_url.as_str())),
//     }
// }

static URL_CHAR_LIMIT: usize = 6000;

//the point of this struct is to get url strings into something we can save and compare with.
//We will assume everything is https, and strip 'www' from everything.
//domain will include the TLD. Will not include the protocall (ie, https)
//Page may be empty string if there are not additional segments after the domain.
//Paramaters aka querries are preserved. So theres a chance that sensitive information or tracking stuff or whatever
//comes with. There may be a smarter way to handle this down the line.
//Also url to same page with different paramaters will not be considered equal. Which may encounter
//some weird edge cases but they should mostly be degenerate and dont propogate.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Link {
    pub domain: String,
    pub page: String,
    pub parameters: String,
}

impl Link {
    pub fn new(wild_link: &String) -> Result<Link, AnyErr> {
        if wild_link.len() > URL_CHAR_LIMIT {
            bail!(
                "Cannot create WebNode with Url longer than {} characters",
                URL_CHAR_LIMIT
            )
        }

        let url = Url::parse(wild_link)?;

        //this condition *should* be covered by the following block of code,
        //but i'm hitting a bug where there seems to be a URL that returns a path
        // with no initial slash when .path() is called, and this would be the condition that
        // would make that possible.
        if url.cannot_be_a_base() {
            bail!("cannot be a base: {}", wild_link)
        }

        let domain = match url.domain() {
            Some(d) => strip_www(&String::from(d)),
            None => {
                bail!("no domain in url");
            }
        };

        let page = String::from(&url.path()[1..=url.path().len() - 1]); //slices the &str to remove the leading '/'

        let parameters = match url.query() {
            Some(params) => String::from(params),
            None => String::new(),
        };

        Ok(Link {
            domain,
            page,
            parameters,
        })
    }

    pub fn as_string(&self) -> String {
        if self.parameters.len() > 0 {
            "https://".to_owned() + &self.domain + "/" + &self.page + "?" + &self.parameters
        } else {
            "https://".to_owned() + &self.domain + "/" + &self.page
        }
    }

    pub fn as_url(&self) -> Url {
        Url::parse(&self.as_string()).unwrap()
    }
}

impl std::fmt::Display for Link {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}
