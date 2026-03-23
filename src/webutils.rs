use std::collections::*;
use std::panic;
use std::time::Duration;

use reqwest::blocking::Client;
use tokio::sync::mpsc;
use tokio::time::timeout;

use reqwest::blocking::Request;
use reqwest::IntoUrl;
reqwest::header::HeaderMap;

use url::form_urlencoded::Parse;
use url::{ParseError, Url};

use select::document::Document;
use select::predicate::Name;
use select::predicate::Predicate;

use texting_robots::{Robot, get_robots_url};

fn get_page_links(body: &str) -> Vec<String> {
    let links = Document::from(body)
        .find(Name("a").or(Name("link")))
        .filter_map(|n| n.attr("href"))
        .map(|n| n.to_string())
        .collect();
    return links;
}

fn link_is_html_from_head(head: reqwest::header::HeaderMap) -> bool {
    if let Some(ct) = head.get("Content-type") {
        let ct_str = ct.to_str().unwrap_or("");
        match ct_str.contains("html") || ct_str.contains("HTML") || ct_str.contains("text") {
            true => {
                return true;
            }
            false => {
                println!(
                    "link skipped because it is not html. Doctype is: {}",
                    ct_str
                );
                return false;
            }
        }
    } else {
        println!("link skipped because it had no content type");
        return false;
    }
}

//todo: I know there is a lot of going back and forth between strings and Url's thats not strictly necessary.
fn normalize_url(url: &String, root: Option<reqwest::Url>) -> Result<String, ParseError> {
    match Url::parse(url) {
        Err(e) => {
            if url.starts_with("/") {
                match root {
                    Some(root_contents) => {
                        let joined = root_contents.join(url)?;
                        normalize_url(&String::from(joined), None)
                    }
                    None => Err(e),
                }
            } else {
                return Err(e);
            }
        }
        Ok(parsed_url) => Ok(String::from(parsed_url.as_str())),
    }
}

fn is_in_domain_list(url: &String, list: &Vec<String>) -> bool {
    match reqwest::Url::parse(url) {
        Ok(parsed_url) => {
            if let Some(domain) = parsed_url.domain() {
                for item in list {
                    if item.contains(&domain) {
                        return true;
                    }
                }
                return false;
            } else {
                // url has root domain, ie, the domain is "/".
                //we will assume for now that is not in any list.
                false
            }
        }
        Err(_) => false, // implicitly, the url is not in the list
    }
}





//if any of the links cant be
fn parse_list(links: Vec<String>) -> Vec< Result<Url, ParseError>> {
    links.into_iter().map(|link| Url::parse(link)?)
}

// fn see_no_evil<T, E>(vals: impl Iterator<Item = Result<T,E>) -> Iterator<Item = T> {

// }

fn see_no_evil<T, E>(vals: Vec<Result<T, E>) -> Vec<T> {
    vals.into_iter().filter_map(|item| item.ok()).collect()
}




fn associated_rbts_txt(link: String) -> Result<Url, ParseError> {
    if let Some(domain) = Url::parse(&link)?.domain() {
        Url::parse(domain)?.join("/robots.txt")
    } else {
        Err(RelativeUrlWithCannotBeABaseBase)
    }
}


//ToDo: there is definetly some code duplication here. But since a govenor is mostly meant to come from a Robots.txt, and they could be useful to fquery outside of that, I
// wanted them to be seperate. For now at least.
pub fn get_robot(link: String, client: reqwest::blocking::Client) -> Result<Robot, ParseError> {
    let rbts_link = associated_rbts_txt(link)?;
    let robots_text: String;

    let mut headers = reqwest::header::HeaderMap::new();
    //ToDo: is this unwrap okay?
    headers.insert("user-agent", "'Mozilla/5.0".parse().unwrap());


    let mut res;
    let mut tries = 0;
    loop {
        res = self.client.get(url).headers(headers).send()?;
        self.last_request = Instant::now();
        let status = res.status();

        if status.is_sucess() {
            robots_text = res.text()?;
            break;
        } else if status.is_redirection() {
            //reqwest handles this for us for a default number (10) of redirect hops. So if we end up here, its exceeded that.
            println!("Error. too many redirects. Url: {url}");
            return Err(status);
        } else if status.is_client_error() {
            println!("Error. Status: {}, Url: {url}", status.as_str());
            return Err(status)
        } else if status.is_server_error() {
            if tries < max_retries {
                tries += 1;
                continue;
            } else {
                return Err(status)
            }
        }
    }

    return Robot::new("SumiCrawler", robots_text.as_bytes());
}

struct Govenor {
    domain: String,
    client: reqwest::blocking::Client,
    //these are pages that i the programmer or the user have blacklisted
    //I intend to maybe eventually switch this over to leveraging the robots.txt machinary, but for now...
    forbidden_pages: Vec<Url>,
    robotstxt: Option<Robot>,
    rate: Duration,
    max_requests: u32,
    total_requests: u32,
    max_retries: u32,
    last_request: std::time::Instanteqwest::blocking::Client::new();
}

impl Default for Govenor {
    fn default() -> self {
        let client = reqwest::blocking::Client::new();
        let forbidden_page_strings = vec![
            "use.typekit.net",
            "cdn.cookielaw.org",
            "assets.adobedtm.com",
        ];
        //ToDo: trust this less? ie, dont `see_no_evil`
        let forbidden_page_urls = see_no_evil(parse_list(forbidden_page_strings));

        rate = Duration::from_secs(2);
        max_requests = 100;
        max_retries = 5;
        total_requests = 0;
        //there has been no request made yet, so set the instant to the UNIX Epoch.
        last_request = Instant::now() - (SystemTime::now() - std::time::UNIX_EPOCH);

        Govenor {client, forbidden_pages, rate, max_requests, max_retries, last_request}
    }
}

impl Govenor {

    pub fn from_link(link: URL) -> Result<Govenor, ParseError> {
        let mut gov = Govenor::default();
        let domain = match link.domain() {
            None => {return Err(RelativeUrlWithCannotBeABaseBase);}
            Some(d) => d
        };

        let mut gov = Govenor::default();
        let robot_txt: Option<Robot> = get_robot(link, gov.client);
        gov.total_requests += 1;
        gov.robotstxt = robot_txt;
        if let Some(robot) = gov.robotstxt {
            gov.rate = robot.delay;
        }

        Ok(gov)
    }
    //returns true if page cannot be parsed into a URL
    pub fn is_page_forbidden(&self, page: String) -> bool {
        let p = Url::parse(&page);
        if p.is_err() {println!("Error. Cannot parse url. {p} ,  Url: {page} "); return true;}
        let page_url = p.unwrap();

        for fpage in self.forbidden_pages {
            //ToDo: I have a feeling this will need to be made more robust and subtle somehow.
            if page_url.domain() == fpage.domain() {
                return true;
            }
        }

        if let Some(robot) = self.robotstxt {
            return !robot.allowed(page);
        }

        return false;
    }

    //all these get methods have automatic retries that are rate limited.
    pub fn get_url(&mut self, url: String) -> Result<String, reqwest::Error> {
        let mut headers = reqwest::header::HeaderMap::new();
        //
        //ToDo: is this unwrap okay?
        headers.insert("user-agent", "'Mozilla/5.0".parse().unwrap());
            let mut res;
            let mut tries = 0;
            loop {
            res = self.client.get(url).headers(headers).send()?;
            self.last_request = Instant::now();
            self.total_requests += 1;
            let status = res.status();

            if status.is_sucess() {
                let body = res.text()?;
                return Ok(body);
            } else if status.is_redirection() {
                //reqwest handles this for us for a default number (10) of redirect hops. So if we end up here, its exceeded that.
                println!("Error. too many redirects. Url: {url}");
                return Err(status);
            } else if status.is_client_error() {
                println!("Error. Status: {}, Url: {url}", status.as_str());
                return Err(status)
            } else if status.is_server_error() {
                if tries < max_retries {
                    tries +=1;
                    continue;
                } else {
                    return Err(status)
                }
            }
        }

    }
    pub fn get_url_head(&mut self, url: String) -> Result<HeaderMap, reqwest::Error> {
        let mut our_headers = reqwest::header::HeaderMap::new();
        our_headers.insert("user-agent", "'Mozilla/5.0".parse().unwrap());

        let mut res;
        let mut tries = 0;
        loop {
            let ret = self.client.head(link).headers(our_headers).send()?;
            self.last_request = Instant::now();
            self.total_requests += 1;
            let status = res.status();

            if status.is_sucess() {
                return Ok(ret.headers().clone());
            } else if status.is_redirection() {
                //reqwest handles this for us for a default number (10) of redirect hops. So if we end up here, its exceeded that.
                println!("Error. too many redirects. Url: {url}");
                return Err(status);
            } else if status.is_client_error() {
                println!("Error. Status: {}, Url: {url}", status.as_str());
                return Err(status)
            } else if status.is_server_error() {
                if tries < max_retries {
                    tries +=1;
                    continue;
                } else {
                    return Err(status);
                }
            }
        }
    }

}
