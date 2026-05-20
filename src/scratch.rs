use std::sync::{Arc, Mutex, PoisonError, RwLock};


use reqwest::{Response, blocking::Request};

fn get(&mut self, link: Link) -> ResponseFuture;

#[derive(Debug)]
pub struct Govenor {
    domain: String, //Has "www." stripped but does have TLD, just like Link
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

static GOV_LOCK_RATE:u64 = 20; // miliseconds

#[derive(Clone)]
struct Gov {
    lock: Arc<Mutex<GovData>>
}

// struct GovInfo {
//     domain: String,
//     rate: Durration,
//     max_requests: u32,
//     max_tries: u32
// }

struct GovData {
    domain: String, //Has "www." stripped but does have TLD, just like Link
    //these are pages that i the programmer or the user have blacklisted
    //I intend to maybe eventually switch this over to leveraging the robots.txt machinary, but for now...
    forbidden_page_urls: Vec<Link>,
    robotstxt: Option<Robot>,
    rate: Duration,
    max_requests: u32,
    total_requests: u32,
    max_tries: u32,
    last_request: std::time::SystemTime,

    // req_queue: Vec<Link>
    last_req: std::future::Future<Output = Response>
}

fn next_req(&mut self, link: Link) -> ResponseFuture {
    self.prev_req.await?;

    request::get().send().await?;
}

impl Gov {
    fn get_v1(&self, link: Link) -> ResponseFut {
        loop 'ACCESS {
            match self.data_1.try_read() {
                Err(_) => {
                    //wait till we can get the lock
                    tokio::time::Sleep(tokio::time::Duration::from_millis(GOV_LOCK_RATE));
                    continue;
                }
                Ok(data) => {
                    let now = std::time::SystemTime::now();
                    let ellapsed = Durration::from(now - data.last_request);
                    if ellapsed < data.rate {
                        tokio::time::Sleep(rate - ellapsed)
                    } else {
                        return self.client.get(link);
                    }
                }
            }
        }
    }

    fn get_v2(&self, link: Link) -> ResponseFut {
        loop 'ACCESS {
            match self.data_2.try_lock() {
                Err(_) => {
                    //wait till we can get the lock
                    tokio::time::Sleep(tokio::time::Duration::from_millis(GOV_LOCK_RATE));
                    continue;
                }
                Ok(data) => {
                    // data.req_queue.push(link);
                    data.
                }
            }
        }
    }

    fn get_v3(&mut self) -> Future<Response> {
        loop 'ACCESS {
            match self.last_future.try_lock() {
                Err(_) => {
                    //wait till we can get the lock
                    tokio::time::Sleep(tokio::time::Duration::from_millis(GOV_LOCK_RATE));
                    continue;
                }
                Ok(future) => {
                    future = {
                        future.await?;
                        self.get()
                    }
                }
            }
        }
    }

    async fn make_request(&mut self, link: Link) -> Response {
        'ACCESS: loop {
            match self.lock.try_lock() {
                Ok(ref mut data) => {
                    return data.chain(link);
                }
                Err(e) => match e {
                    WouldBlock => {tokio::time::sleep(GOV_LOCK_RATE);}
                    PoisonError => {panic!("mutex is poisoned for govenor");}
                }
            }
        }
    }
}

impl GovData {
    async fn chain(&mut self, link: Link) -> Request {
        let _ = self.last_req.await?;
        self.last_req = self.get(link, false);
        self.last_req.clone();
    }
}






#[derive(Clone)]
pub struct Bureaucracy {
    govs: Arc<RwLock<HashMap<String, Govenor>>>,
    client: reqwest::Client,
}


#[derive(Debug, Clone)]
pub struct Govenor {
    pub lock: Arc<Mutex<GovData>>,
}

#[derive(Debug)]
pub struct GovData {
    domain: String, //Has "www." stripped but does have TLD, just like Link
    client: reqwest::Client,
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
