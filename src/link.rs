use crate::error::*;
use crate::utils::strip_www;
use url::{ParseError, Url};

use select::document::Document;
use select::predicate::Name;
use select::predicate::Predicate;

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
//additional params will attempt to be stripped from the end. Eg, passwords and tracking params etc.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Link {
    pub domain: String,
    pub page: String,
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
        let domain = match url.domain() {
            Some(d) => strip_www(&String::from(d)),
            None => {
                bail!("no domain in url");
            }
        };

        let page = String::from(&url.path()[1..=url.path().len() - 1]); //slices the &str to remove the leading '/'
        Ok(Link { domain, page })
    }

    pub fn as_string(&self) -> String {
        "https://".to_owned() + &self.domain + "/" + &self.page
    }

    pub fn as_url(&self) -> Url {
        Url::parse(&("https://".to_owned() + &self.as_string())).unwrap()
    }
}

pub fn get_page_links(body: &str) -> Vec<Link> {
    let links = Document::from(body)
        .find(Name("a").or(Name("link")))
        .filter_map(|n| n.attr("href"))
        .map(|n| Link::new(&n.to_string()))
        .filter_map(|res| res.ok())
        .collect();
    return links;
}
