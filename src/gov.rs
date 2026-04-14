use crate::utils::*;
use crate::{link::*, utils::*, AnyErr};
use anyhow::bail;
use lazy_static::lazy_static;
use std::collections::*;
use std::time::{Duration, Instant, SystemTime};
use texting_robots::{get_robots_url, Robot};
use url::Url;

lazy_static! {
    //ToDo: make this more robust
    pub static ref STUBS: Vec<Link> = vec![
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
    .filter_map(|s| Link::new(&s).ok())
    .collect();

    //ToDo: make this more robust
    pub static ref BLACKLIST: Vec<Link> = vec![
        "typekit.net",
        "cookielaw.org",
        "adobedtm.com",
        "adobe.com",
        "uservoice.com",
        "googleapis.com",

    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .filter_map(|s| Link::new(&s).ok())
    .collect();
}

#[derive(Debug)]
pub struct Govenor {
    domain: String, //Has "www." stripped but does have TLD, just like Link
    client: reqwest::blocking::Client,
    //these are pages that i the programmer or the user have blacklisted
    //I intend to maybe eventually switch this over to leveraging the robots.txt machinary, but for now...
    forbidden_page_urls: Vec<Link>,
    robotstxt: Option<Robot>,
    rate: Duration,
    max_requests: u32,
    total_requests: u32,
    max_retries: u32,
    last_request: std::time::SystemTime,
}

impl Default for Govenor {
    fn default() -> Self {
        //Todo, set this to the past rather than now. That way when we try to make the first request, we don't have to wait
        Govenor {
            domain: String::new(),
            client: reqwest::blocking::Client::new(),
            //ToDo: trust this less? ie, dont `see_no_evil`
            forbidden_page_urls: BLACKLIST.clone(),
            rate: Duration::from_secs(1),
            max_requests: 50,
            max_retries: 5,
            total_requests: 0,
            robotstxt: None,
            last_request: SystemTime::now(),
        }
    }
}

impl Govenor {
    pub fn from_domain(domain: &String) -> Result<Govenor, AnyErr> {
        let mut gov = Govenor::default();
        gov.domain = domain.clone();

        match gov.get_robot() {
            Ok(robot) => {
                if let Some(delay) = robot.delay {
                    gov.rate = Duration::from_secs_f32(delay);
                }
                gov.robotstxt = Some(robot);
            }
            Err(e) => {
                //ToDo: there are more error cases here than no robots.txt. Handle better?
                println!("no robots.txt for {domain}");
            }
        }
        Ok(gov)
    }

    pub fn get_robot(&mut self) -> Result<Robot, AnyErr> {
        let rbts_link = get_robots_url(&self.as_domain_str())?;
        let robots_text = self.get_url(&String::from(rbts_link.as_str()), false)?;
        return Robot::new("SumiCrawler", robots_text.as_bytes());
    }

    pub fn as_domain_str(&self) -> String {
        String::from("https://") + &self.domain
    }

    pub fn get_url_to_domain(&self) -> Url {
        Url::parse(&(String::from("https://") + &self.domain)).unwrap()
    }

    //
    pub fn page_is_forbidden(&self, page: &Link) -> bool {
        for fpage in &self.forbidden_page_urls {
            //ToDo: make sure this handles all edge cases
            if page.as_string().contains(&fpage.as_string()) {
                return true;
            }
        }

        if let Some(robot) = &self.robotstxt {
            return !robot.allowed(&page.as_string());
        }

        return false;
    }

    //has automatic retries that are rate limited.
    //if only_head is true, only requests the head for that url and returns the content type of the header, or empty string if there is none.
    pub fn get_url(&mut self, url: &String, only_head: bool) -> Result<String, AnyErr> {
        let link = Link::new(url)?;
        if self.page_is_forbidden(&link) {
            println!("cannot get that page, it is fobidden!");
            bail!("page is forbidden: {}", url);
        }

        let mut headers = reqwest::header::HeaderMap::new();
        //
        //ToDo: is this unwrap okay?
        headers.insert("user-agent", "'Mozilla/5.0".parse().unwrap());

        let ellapsed = self.last_request.elapsed()?;
        let sleep_len = if ellapsed > self.rate {
            Duration::from_secs(0)
        } else {
            self.rate - ellapsed
        };
        std::thread::sleep(sleep_len);

        let mut tries = 0;
        loop {
            let request = match only_head {
                false => self.client.get(url).headers(headers.clone()),
                true => self.client.head(url).headers(headers.clone()),
            };
            let response = request.send()?;
            self.last_request = SystemTime::now();
            self.total_requests += 1;
            let status = response.status();

            if status.is_success() {
                if only_head {
                    match response.headers().get("content-type") {
                        Some(v) => return Ok(String::from(v.to_str()?)),
                        None => {
                            return Ok(String::new());
                        } //ToDo: Should this bail instead?
                    }
                }
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

    //Todo, when this doesnt need the error reporting, it can be reduced to one line
    pub fn content_type_is_html(content_type: &String) -> bool {
        match content_type.contains("html")
            || content_type.contains("HTML")
            || content_type.contains("text")
        {
            true => {
                return true;
            }
            false => {
                println!(
                    "link skipped because it is not html. Doctype is: {}",
                    content_type
                );
                return false;
            }
        }
    }

    pub fn html_links_from_page_body(&mut self, body: String) -> Vec<Link> {
        let links = get_page_links(&body);
        let mut html_links = Vec::with_capacity(links.len());

        for link in links {
            if let Some(ext) = get_ext(&link.as_string()) {
                if ext != ".html" || ext != ".txt" || ext != ".rtf" || ext != ".xml" {
                    continue;
                }
            }
            let content_type = match self.get_url(&link.as_string(), true) {
                Ok(h) => h,
                Err(e) => {
                    println!("{e}");
                    continue;
                } //ToDo: collect these errors?
            };
            if Govenor::content_type_is_html(&content_type) {
                html_links.push(link);
            }
        }

        html_links
    }
}
