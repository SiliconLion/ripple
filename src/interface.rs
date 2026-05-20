use crate::gov::*;
use crate::link::*;
use crate::utils::*;
use futures::executor;
use std::sync::Arc;
use tokio::task::JoinSet;

// use tokio::

#[derive(Debug)]
pub enum Action {
    Explore,
    Validate,
}

pub struct ActionResult {
    pub link: Link,
    pub state: CrawlState,
}

impl ActionResult {
    pub fn new(link: Link, state: CrawlState) -> ActionResult {
        ActionResult { link, state }
    }
}

pub trait Strategy {
    fn next_nodes(&mut self, data: &Box<dyn Data>) -> Vec<(Action, Link)>;
    fn end(&mut self, data: &Box<dyn Data>) -> bool;
    fn max_poll_frequency(&self) -> std::time::Duration;
}

pub trait Selector: Send + Sync {
    fn extract_canidates(&self, text: &String) -> Vec<Link>;
}

pub trait Authenticator: Send + Sync {
    // fn is_valid(&self, header: &reqwest::header::HeaderMap) -> bool;
    fn is_valid_from_content_type(&self, content_type: &String) -> bool;
}

#[derive(Clone, Debug)]
pub enum CrawlState {
    Canidate,
    Stub,                // we do not explore this one but we do surface it
    Forbidden,           // link is blacklisted
    Rejected,            // link turns out not to be one we are interested in
    Verified,            // we know link is html or txt page
    Explored(Vec<Link>), //a vec of all the canidate links found in the response of the page
    Failed,              //ToDo: status code of failure to load page
}

/*
                      (   )
                        |          // Add Canidate
                        v
                  +------------+
                  | Candidate  | -----------------+
                  +------------+                  |
                 /      |        \                |     // Verify action
               v        v         v               v
       +--------+ +-----------+ +-----------+ +-----------+
       | Stub   | | Forbidden | | Verified  | | Rejected  |
       +--------+ +-----------+ +-----------+ +-----------+
                                   /     \
                                  v       v                  // Expore action
                           +-----------+  +--------+
                           | Explored  |  | Failed |
                           +-----------+  +--------+
*/

#[derive(Clone, Debug)]
pub struct WebNode {
    pub link: Link,
    pub state: CrawlState,
}

impl WebNode {
    pub fn new_canidate(link: &Link) -> WebNode {
        WebNode {
            link: link.clone(),
            state: CrawlState::Canidate,
        }
    }
}

pub trait Data {
    fn get(&self, link: &Link) -> &WebNode;
    // fn get_mut(&mut self, link: &Link) -> Result<&mut WebNode, AnyErr>;
    fn add(&mut self, node: WebNode);
    fn remove(&mut self, link: Link);
    fn neighbors(&self, link: Link) -> Vec<Link>;
    fn all_nodes(&self) -> Vec<Link>;
    fn domain_names(&self) -> Vec<String>;
    fn get_domain(&self, domain: String) -> Vec<Link>;
    // fn roots(&self) -> Vec<Link>;
    fn update(&mut self, res: ActionResult);
    fn represent(&self) -> String;
}

pub struct Application {
    pub strategy: Box<dyn Strategy>,
    pub selector: Arc<dyn Selector>,
    pub auth: Arc<dyn Authenticator>,
    pub data: Box<dyn Data>,
    bureau: Bureaucracy,
}

impl Application {
    pub fn new(
        strategy: Box<dyn Strategy>,
        selector: Arc<dyn Selector>,
        auth: Arc<dyn Authenticator>,
        data: Box<dyn Data>,
    ) -> Application {
        Application {
            strategy,
            selector,
            auth,
            data,
            bureau: Bureaucracy::new(),
        }
    }

    pub fn start(&mut self, root_link: Link) -> Result<(), AnyErr> {
        self.data.add(WebNode {
            link: root_link,
            state: CrawlState::Canidate,
        });
        self.work()
    }

    pub fn work(&mut self) -> Result<(), AnyErr> {
        while !self.strategy.end(&self.data) {
            let pass_start_time = std::time::Instant::now();
            let next_nodes = self.strategy.next_nodes(&self.data);

            let mut set = JoinSet::new();
            for (action, link) in next_nodes {
                let selector = self.selector.clone();
                let auth = self.auth.clone();
                let bureau = self.bureau.clone();

                set.spawn(async move {
                    use Action::*;
                    use CrawlState::*;
                    let res = match action {
                        Explore => {
                            let resp = bureau.get_url(&link, false).await; //await here
                            if resp.is_err() {
                                ActionResult::new(link, Failed)
                            } else {
                                let body = resp.unwrap();
                                let canidates = selector.extract_canidates(&body);
                                ActionResult::new(link, Explored(canidates))
                            }
                        }
                        Validate => {
                            let resp = bureau.get_url(&link, true).await; //await here
                            if resp.is_err() {
                                ActionResult::new(link, Failed)
                            } else {
                                let ct = resp.unwrap();
                                match auth.is_valid_from_content_type(&ct) {
                                    true => ActionResult::new(link, Verified),
                                    false => ActionResult::new(link, Rejected),
                                }
                            }
                        }
                    };
                    res
                });
            }

            let action_results = executor::block_on(set.join_all());
            for ar in action_results {
                self.data.update(ar);
            }

            let pass_end_time = std::time::Instant::now();

            let delta = pass_end_time - pass_start_time;
            if delta < self.strategy.max_poll_frequency() {
                std::thread::sleep(self.strategy.max_poll_frequency() - delta);
            }
        }

        return Ok(());
    }
}
