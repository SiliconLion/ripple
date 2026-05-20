use crate::utils::*;
use crate::{link::*, utils::*};
use anyhow::bail;
use futures::future::FutureExt;
use lazy_static::lazy_static;
use std::collections::*;
use std::future::Future;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};
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

// #[derive(Clone)]
// pub struct Govenor {
//     data: Arc<GovData>,
// }

// impl Govenor {
//     pub fn from_domain(domain: &String) -> Result<Govenor, AnyErr> {
//         Ok(Govenor {
//             data: Arc::new(GovData::from_domain(domain)?),
//         })
//     }

//     async fn get(&mut self, link: &Link, only_head: bool) -> Result<String, AnyErr> {
//         let mut data = open_mutex(&*self.lock);
//         data.get(link, only_head).await
//     }
// }

#[derive(Clone)]
pub struct Govenor {
    domain: String, //Has "www." stripped but does have TLD, just like Link
    client: reqwest::Client,
    //these are pages that i the programmer or the user have blacklisted
    //I intend to maybe eventually switch this over to leveraging the robots.txt machinary, but for now...
    forbidden_page_urls: Vec<Link>,
    robotstxt: Arc<Option<Robot>>,
    rate: Duration,
    max_requests: u32,
    max_tries: u32,

    lock: Arc<Mutex<GovMutData>>,
}

// #[derive(Debug)]
pub struct GovMutData {
    total_requests: u32,
    last_req_time: std::time::SystemTime,
    // last_req: Option<Arc<dyn Future<Output = Result<reqwest::Response, reqwest::Error>>>>,
}

impl Default for GovMutData {
    fn default() -> Self {
        GovMutData {
            total_requests: 0,
            last_req_time: std::time::UNIX_EPOCH, //just putting it far in the past as there have been no request made yet
        }
    }
}

impl Default for Govenor {
    fn default() -> Self {
        //Todo, set this to the past rather than now. That way when we try to make the first request, we don't have to wait
        Govenor {
            domain: String::new(),
            client: reqwest::Client::new(),
            forbidden_page_urls: BLACKLIST.clone(),
            robotstxt: Arc::new(None),
            rate: Duration::from_secs(1),
            max_requests: 50,
            max_tries: 3,
            lock: Arc::new(Mutex::new(GovMutData::default())),
        }
    }
}

impl Govenor {
    pub fn from_domain(domain: &String) -> Result<Govenor, AnyErr> {
        // let mut gov = Govenor::default();
        // gov.domain = domain.clone();

        // match gov.get_robot() {
        //     Ok(robot) => {
        //         if let Some(delay) = robot.delay {
        //             gov.rate = Duration::from_secs_f32(delay);
        //         }
        //         gov.robotstxt = Some(robot);
        //     }
        //     Err(e) => {
        //         //ToDo: there are more error cases here than no robots.txt. Handle better?
        //         println!("{e}");
        //     }
        // }
        // Ok(gov)

        // //this suuuck and the above code would be fine if I could clone a Robot. But unfortunately cant and until the issue gets approved in the
        // //texting robots repo, this is how its gonna be.
        // //
        let mut gov = Govenor::default();
        gov.domain = domain.clone();

        match gov.get_robot() {
            Ok(robot) => {
                if let Some(delay) = robot.delay {
                    gov.rate = Duration::from_secs_f32(delay);
                }

                Ok(Govenor {
                    domain: gov.domain,
                    forbidden_page_urls: gov.forbidden_page_urls.clone(),
                    rate: gov.rate,
                    max_requests: gov.max_requests,
                    max_tries: gov.max_tries,
                    robotstxt: Arc::new(Some(robot)), //this line is the *WHOLE* reason for this awkward line by line copying.
                    client: gov.client,
                    lock: gov.lock,
                })
            }
            Err(e) => {
                //ToDo: there are more error cases here than no robots.txt. Handle better?
                println!("{e}");
                return Ok(gov);
            }
        }
    }

    pub fn get_robot(&mut self) -> Result<Robot, AnyErr> {
        let handle = tokio::runtime::Handle::current();
        let robots_text = handle.block_on(async {
            let rbts_link = Link::new(&get_robots_url(&self.as_domain_str())?)?;
            self.get(&rbts_link, false).await
        })?;

        return Robot::new("SumiCrawler", robots_text.as_bytes());
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

        if let Some(robot) = &*self.robotstxt {
            return !robot.allowed(&page.as_string());
        }

        return false;
    }

    async fn get(&self, link: &Link, only_head: bool) -> Result<String, AnyErr> {
        //     let last_req = {
        //         let mut_data = open_mutex(&self.lock);
        //         mut_data.last_req.clone()
        //         //mutex locks again
        //     };
        //     if let Some(request) = last_req {
        //         (*request).await?;
        //     }
        self.the_whole_get_machinery(link, only_head).await
    }

    async fn the_whole_get_machinery(
        &self,
        link: &Link,
        only_head: bool,
    ) -> Result<String, AnyErr> {
        if self.page_is_forbidden(&link) {
            println!("cannot get that page, it is fobidden!");
            bail!("page is forbidden: {}", link);
        }

        //The the way this will all get scheduled out probably sucks but it just needs to
        // work right now, and it will get us through being *SO* IO bound.
        'TIMING: loop {
            let last_req_time = {
                let mut_data = open_mutex(&self.lock);
                mut_data.last_req_time.clone()
                //mutex relocks
            };

            let ellapsed = last_req_time.elapsed()?;
            if ellapsed > self.rate {
                break 'TIMING;
            } else {
                tokio::time::sleep(self.rate - ellapsed);
            };
        }

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("user-agent", "'Mozilla/5.0".parse().unwrap());

        let mut tries = 0;
        loop {
            let total_requests = {
                let mut_data = open_mutex(&self.lock);
                mut_data.total_requests.clone()
                //mutex relocks
            };
            if total_requests >= self.max_requests {
                bail! {"max requests reached for domain"}
            }

            let request = match only_head {
                false => self.client.get(link.as_url()).headers(headers.clone()),
                true => self.client.head(link.as_url()).headers(headers.clone()),
            }
            .send();
            // .shared();

            {
                let mut mut_data = open_mutex(&self.lock);
                mut_data.last_req_time = SystemTime::now();
                mut_data.total_requests += 1;
                // mut_data.last_req = Some(Arc::new(request.clone()))
                //mutex relocks
            }
            let response = request.await?;
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
                let body = response.text().await?;
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

#[derive(Clone)]
pub struct Bureaucracy {
    //ToDo: this maybe should be a RwLock instead
    govs: HashMap<String, Govenor>,
    // client: reqwest::Client,
}

unsafe impl Send for Bureaucracy {}
unsafe impl Sync for Bureaucracy {}

impl Bureaucracy {
    pub fn new() -> Bureaucracy {
        Bureaucracy {
            govs: HashMap::new(),
            // client: reqwest::Client::new(),
        }
    }
    pub fn add_gov(&mut self, domain: &String) -> Result<(), AnyErr> {
        if self.govs.contains_key(domain) {
            bail! {"Govenor with that key already exists"};
        }
        let gov = Govenor::from_domain(domain)?;
        self.govs.insert(domain.clone(), gov);
        Ok(())
    }

    pub fn get_gov(&self, domain: &String) -> Option<&Govenor> {
        self.govs.get(domain)
    }
    pub fn get_gov_mut(&mut self, domain: &String) -> Option<&mut Govenor> {
        self.govs.get_mut(domain)
    }
    pub fn get_gov_or_add(&mut self, link: &Link) -> Result<&mut Govenor, AnyErr> {
        if !self.govs.contains_key(&link.domain) {
            self.add_gov(&link.domain)?;
        }
        Ok(self.get_gov_mut(&link.domain).unwrap())
    }

    //has automatic retries that are rate limited.
    //if only_head is true, only requests the head for that url and returns the content type of the header, or empty string if there is none.
    pub async fn get_url(&self, link: &Link, only_head: bool) -> Result<String, AnyErr> {
        let gov = self.get_gov(&link.domain).unwrap();
        gov.get(link, only_head).await
    }
}
