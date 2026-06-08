use crate::utils::*;
use crate::{link::*, utils::*};
use anyhow::bail;
use futures::channel;
use lazy_static::lazy_static;
use std::collections::*;
use std::time::{Duration, Instant, SystemTime};
use texting_robots::{get_robots_url, Robot};
use url::Url;

use std::sync::mpsc::{channel, Receiver, Sender};

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

pub struct Submission {
    //ToDo: make this a one shot rather than a full channel
    sender: Sender<Result<String, AnyErr>>,
    page: Link,
    head_only: bool,
}

// I know im 98% of the way to recreating futures, but then I need a channel to pass
// the waker to the Govenor each time poll is called, and then theres a bunch of control flow reasons
// that it would spagettify the code for no benifit. When I need it, Ill impliment it and then I can call "await" rather than calling "is_ready"
// in a loop.
pub struct Reply {
    //ToDo: make this a one shot rather than a full channel
    pub reciver: Receiver<Result<String, AnyErr>>,
}

pub fn new_pair(page: &Link, head_only: bool) -> (Submission, Reply) {
    let (tx, rx) = channel();
    (
        Submission {
            sender: tx,
            page: page.clone(),
            head_only,
        },
        Reply { reciver: rx },
    )
}

#[derive(Clone)]
pub struct GovHandle {
    // core: Arc<Mutex<GovCore>>
    pub sender: std::sync::mpsc::Sender<Submission>,
    pub domain: String,
}

impl GovHandle {
    pub fn request(&self, page: &Link, head_only: bool) -> Reply {
        let (submission, reply) = new_pair(page, head_only);
        self.sender.send(submission);
        reply
    }
}

pub struct Govenor {
    pub recv: Receiver<Submission>,
    pub core: GovenorCore,
}

impl Govenor {
    pub fn create_govenor(domain: &String) -> GovHandle {
        let (sender, recv) = channel();
        let govenor = Govenor {
            core: GovenorCore::from_domain(domain),
            recv,
        };

        govenor.start(); //moves the govenor into its own thread
        GovHandle {
            domain: domain.clone(),
            sender,
        }
    }

    pub fn start(mut self) {
        std::thread::Builder::new()
            .name(format!("{}-govenor", self.core.domain))
            .spawn(move || loop {
                match self.recv.recv() {
                    Err(e) => {
                        println!("Error: {e}");
                        break;
                    }
                    Ok(submission) => {
                        let res = self.core.get(&submission.page, submission.head_only);
                        submission.sender.send(res);
                    }
                }
            });
    }
}

#[derive(Debug)]
pub struct GovenorCore {
    domain: String, //Has "www." stripped but does have TLD, just like Link
    client: reqwest::blocking::Client,
    //these are pages that i the programmer or the user have blacklisted
    //I intend to maybe eventually switch this over to leveraging the robots.txt machinary, but for now...
    forbidden_page_urls: Vec<Link>,
    robotstxt: Option<Robot>,
    rate: Duration,
    max_requests: u32,
    total_requests: u32,
    max_tries: u32,
    last_request: std::time::SystemTime,
}

impl Default for GovenorCore {
    fn default() -> Self {
        //Todo, set this to the past rather than now. That way when we try to make the first request, we don't have to wait
        GovenorCore {
            domain: String::new(),
            forbidden_page_urls: BLACKLIST.clone(),
            rate: Duration::from_secs(1),
            max_requests: 50,
            max_tries: 3,
            total_requests: 0,
            robotstxt: None,
            last_request: SystemTime::now(),
            client: reqwest::blocking::Client::new(),
        }
    }
}

impl GovenorCore {
    pub fn from_domain(domain: &String) -> GovenorCore {
        let mut gov = GovenorCore::default();
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
                println!("{e}");
            }
        }
        gov
    }

    pub fn get_robot(&mut self) -> Result<Robot, AnyErr> {
        let rbts_link = Link::new(&get_robots_url(&self.as_domain_str())?)?;
        println!("{rbts_link:?}");
        let robots_text = self.get(&rbts_link, false)?;
        let r = Robot::new("SumiCrawler", robots_text.as_bytes());
        // println!("{:?}", r);
        r
    }

    pub fn as_domain_str(&self) -> String {
        String::from("https://") + &self.domain
    }

    pub fn get_url_to_domain(&self) -> Url {
        Url::parse(&(String::from("https://") + &self.domain)).unwrap()
    }

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

    fn get(&mut self, link: &Link, only_head: bool) -> Result<String, AnyErr> {
        if self.page_is_forbidden(&link) {
            println!("cannot get that page, it is fobidden!");
            bail!("page is forbidden: {}", link);
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
            if self.total_requests == self.max_requests {
                bail! {"max requests reached for domain"}
            }
            let request = match only_head {
                false => self.client.get(link.as_url()).headers(headers.clone()),
                true => self.client.head(link.as_url()).headers(headers.clone()),
            };
            let response = request.send()?;
            self.last_request = SystemTime::now();
            self.total_requests += 1;
            tries += 1;
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
                println!("Error. too many redirects. Url: {link}");
                bail!(status);
            } else if status.is_client_error() {
                println!("Error. Status: {}, Url: {link}", status.as_str());
                bail!(status);
            } else if status.is_server_error() {
                if tries < self.max_tries {
                    //will retry
                    continue;
                } else {
                    println!("exceeded max retries for domain.");
                    bail!(status);
                }
            }
        }
    }
}

pub struct Bureaucracy {
    govs: HashMap<String, GovHandle>,
}

impl Bureaucracy {
    pub fn new() -> Bureaucracy {
        Bureaucracy {
            govs: HashMap::new(),
        }
    }
    pub fn add_gov(&mut self, domain: &String) -> Result<(), AnyErr> {
        if self.govs.contains_key(domain) {
            bail! {"Govenor with that key already exists"};
        }
        let gov = Govenor::create_govenor(domain);
        self.govs.insert(domain.clone(), gov);
        Ok(())
    }

    pub fn get_gov(&self, domain: &String) -> Option<&GovHandle> {
        self.govs.get(domain)
    }
    pub fn get_gov_mut(&mut self, domain: &String) -> Option<&mut GovHandle> {
        self.govs.get_mut(domain)
    }
    pub fn get_gov_or_add(&mut self, link: &Link) -> Result<&mut GovHandle, AnyErr> {
        if !self.govs.contains_key(&link.domain) {
            self.add_gov(&link.domain)?;
        }
        Ok(self.get_gov_mut(&link.domain).unwrap())
    }

    //has automatic retries that are rate limited.
    //if only_head is true, only requests the head for that url and returns the content type of the header, or empty string if there is none.
    pub fn request(&mut self, link: &Link, only_head: bool) -> Result<Reply, AnyErr> {
        // let c_clone = self.client.clone();
        let gov = self.get_gov_or_add(&link)?;
        // gov.get(link, only_head, c_clone)
        Ok(gov.request(link, only_head))
    }
}
