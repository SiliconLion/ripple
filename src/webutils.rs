use std::collections::*;
use std::time::{Duration, Instant, SystemTime};

use reqwest::header::HeaderMap;
use url::{ParseError, Url};

use select::document::Document;
use select::predicate::Name;
use select::predicate::Predicate;

use texting_robots::{get_robots_url, Robot};

use anyhow::{bail, Context};
use lazy_static::lazy_static;

type AnyErr = anyhow::Error;

lazy_static! {
    //ToDo: make this more robust
    //these lists should perhaps be Arc<Vec<String>>?
    static ref STUBS: Vec<String> = vec![
        "facebook.com",
        "youtube.com",
        "instagram.com",
        "x.com",
        "twitter.com",
        "stackoverflow.com",
        "adobe.com",
        "patreon.com",
        "wikipedia.com",
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect();

    //ToDo: make this more robust
    static ref BLACKLIST: Vec<String> = vec![
        "use.typekit.net",
        "cdn.cookielaw.org",
        "assets.adobedtm.com",
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect();
}

fn get_page_links(body: &str) -> Vec<String> {
    let links = Document::from(body)
        .find(Name("a").or(Name("link")))
        .filter_map(|n| n.attr("href"))
        .map(|n| n.to_string())
        .collect();
    return links;
}

//todo: I know there is a lot of going back and forth between strings and Url's thats not strictly necessary.
fn normalize_url(url: &String, root: Option<reqwest::Url>) -> Result<String, AnyErr> {
    match Url::parse(url) {
        Err(e) => {
            if url.starts_with("/") {
                match root {
                    Some(root_contents) => {
                        let joined = root_contents.join(url)?;
                        normalize_url(&String::from(joined), None)
                    }
                    None => Err(From::from(e)),
                }
            } else {
                return Err(From::from(e));
            }
        }
        Ok(parsed_url) => Ok(String::from(parsed_url.as_str())),
    }
}

pub fn have_same_domain(link1: &String, link2: &String) -> bool {
    let link1_url = match Url::parse(link1) {
        Err(_) => {
            return false;
        }
        Ok(url) => url,
    };
    let link2_url = match Url::parse(link2) {
        Err(_) => {
            return false;
        }
        Ok(url) => url,
    };
    let domain1 = match link1_url.domain() {
        Some(dom) => dom,
        None => {
            return false;
        }
    };
    let domain2 = match link2_url.domain() {
        Some(dom) => dom,
        None => {
            return false;
        }
    };
    return domain1 == domain2;
}

fn is_in_domain_list(url: &String, list: &Vec<String>) -> bool {
    for link in list {
        if have_same_domain(url, link) {
            return true;
        }
    }
    return false;
}

//if any of the links cant be parsed,
fn parse_list(links: Vec<String>) -> Vec<Result<Url, ParseError>> {
    links.into_iter().map(|link| Url::parse(&link)).collect()
}

// fn see_no_evil<T, E>(vals: impl Iterator<Item = Result<T,E>) -> Iterator<Item = T> {

// }

fn see_no_evil<T, E>(vals: Vec<Result<T, E>>) -> Vec<T> {
    vals.into_iter().filter_map(|item| item.ok()).collect()
}

fn associated_rbts_txt(link: String) -> Result<Url, AnyErr> {
    if let Some(domain) = Url::parse(&link)?.domain() {
        Ok(Url::parse(domain)?.join("/robots.txt")?)
    } else {
        bail!("link has no domain. relative url? try normalizing first?")
    }
}

#[derive(Debug)]
pub struct Govenor {
    domain: String,
    client: reqwest::blocking::Client,
    //these are pages that i the programmer or the user have blacklisted
    //I intend to maybe eventually switch this over to leveraging the robots.txt machinary, but for now...
    forbidden_page_urls: Vec<Url>,
    robotstxt: Option<Robot>,
    rate: Duration,
    max_requests: u32,
    total_requests: u32,
    max_retries: u32,
    last_request: std::time::Instant,
}

impl Default for Govenor {
    fn default() -> Self {
        //Todo, set this to the past rather than now. That way when we try to make the first request, we don't have to wait
        Govenor {
            domain: String::new(),
            client: reqwest::blocking::Client::new(),
            //ToDo: trust this less? ie, dont `see_no_evil`
            forbidden_page_urls: see_no_evil(parse_list(BLACKLIST.clone())),
            rate: Duration::from_secs(2),
            max_requests: 50,
            max_retries: 5,
            total_requests: 0,
            robotstxt: None,
            last_request: Instant::now(),
        }
    }
}

impl Govenor {
    pub fn from_link(link: Url) -> Result<Govenor, AnyErr> {
        let mut gov = Govenor::default();
        if let Some(domain) = link.domain() {
            gov.domain = domain.to_string();
        } else {
            bail!("link does not have domain");
        }

        match gov.get_robot() {
            Ok(robot) => {
                if let Some(delay) = robot.delay {
                    gov.rate = Duration::from_secs_f32(delay);
                }
                gov.robotstxt = Some(robot);
            }
            Err(e) => {
                bail!("Error: Cannot get robots.txt : {e}");
            }
        }
        Ok(gov)
    }

    pub fn get_robot(&mut self) -> Result<Robot, AnyErr> {
        let rbts_link = associated_rbts_txt(self.domain.clone())?;
        let robots_text = self.get_url(&String::from(rbts_link.as_str()))?;
        return Robot::new("SumiCrawler", robots_text.as_bytes());
    }

    //returns true if page cannot be parsed into a URL
    pub fn is_page_forbidden(&self, page: String) -> bool {
        let p = Url::parse(&page);
        if p.is_err() {
            println!("Error. Cannot parse url. {} ,  Url: {page} ", page);
            return true;
        }
        let page_url = p.unwrap();

        for fpage in &self.forbidden_page_urls {
            //ToDo: I have a feeling this will need to be made more robust and subtle somehow.
            if page_url.domain() == fpage.domain() {
                return true;
            }
        }

        if let Some(robot) = &self.robotstxt {
            return !robot.allowed(&page);
        }

        return false;
    }

    //all these get methods have automatic retries that are rate limited.
    pub fn get_url(&mut self, url: &String) -> Result<String, AnyErr> {
        let mut headers = reqwest::header::HeaderMap::new();
        //
        //ToDo: is this unwrap okay?
        headers.insert("user-agent", "'Mozilla/5.0".parse().unwrap());

        let now = Instant::now();
        let sleep_len = std::cmp::max(
            Duration::from_millis(0),
            self.rate - now.duration_since(self.last_request),
        );
        std::thread::sleep(sleep_len);

        let mut tries = 0;
        loop {
            let request = self.client.get(url).headers(headers.clone());
            let response = request.send()?;
            self.last_request = Instant::now();
            self.total_requests += 1;
            let status = response.status();

            if status.is_success() {
                let body = response.text()?;
                return Ok(body);
            } else if status.is_redirection() {
                //reqwest handles this for us for a default number (10) of redirect hops. So if we end up here, its exceeded that.
                println!("Error. too many redirects. Url: {url}");
                bail!(status);
            } else if status.is_client_error() {
                println!("Error. Status: {}, Url: {url}", status.as_str());
                bail!(status);
            } else if status.is_server_error() {
                if tries < self.max_retries {
                    tries += 1;
                    continue;
                } else {
                    println!("exceeded max retries for domain.");
                    bail!(status);
                }
            }
        }
    }

    pub fn get_url_head(&mut self, link: String) -> Result<HeaderMap, AnyErr> {
        // let mut our_headers = reqwest::header::HeaderMap::new();
        // our_headers.insert("user-agent", "'Mozilla/5.0".parse().unwrap());

        // let mut res: u32;
        // let mut tries = 0;
        // loop {
        //     let ret = self.client.head(link).headers(our_headers).send()?;
        //     self.last_request = Instant::now();
        //     self.total_requests += 1;
        //     let status = res.status();

        //     if status.is_sucess() {
        //         return Ok(ret.headers().clone());
        //     } else if status.is_redirection() {
        //         //reqwest handles this for us for a default number (10) of redirect hops. So if we end up here, its exceeded that.
        //         println!("Error. too many redirects. Url: {url}");
        //         return Err(status);
        //     } else if status.is_client_error() {
        //         println!("Error. Status: {}, Url: {url}", status.as_str());
        //         return Err(status);
        //     } else if status.is_server_error() {
        //         if tries < self.max_retries {
        //             tries += 1;
        //             continue;
        //         } else {
        //             return Err(status);
        //         }
        //     }
        // }
        unimplemented!()
        //Todo: unifiy requests into one request with retries method to reduce code duplication?
    }

    pub fn get_sitemap(&self) -> Option<Url> {
        unimplemented!()
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

    pub fn html_links_from_page_body(&mut self, body: String) -> Vec<String> {
        let links = get_page_links(&body);
        let mut html_links = Vec::with_capacity(links.len());

        for link in links {
            let head = match self.get_url_head(link.clone()) {
                Ok(h) => h,
                Err(e) => {
                    println!("{e}");
                    continue;
                } //ToDo: collect these errors?
            };
            if Govenor::link_is_html_from_head(head) {
                html_links.push(link);
            }
        }

        html_links
    }
}
