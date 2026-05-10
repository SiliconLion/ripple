use crate::error::*;
use crate::gov::*;
use crate::link::*;

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

pub trait Selector {
    fn extract_canidates(&self, text: &String) -> Vec<Link>;
}

pub trait Authenticator {
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
    pub selector: Box<dyn Selector>,
    pub auth: Box<dyn Authenticator>,
    pub data: Box<dyn Data>,
    bureou: Bureaucracy,
}

impl Application {
    pub fn new(
        strategy: Box<dyn Strategy>,
        selector: Box<dyn Selector>,
        auth: Box<dyn Authenticator>,
        data: Box<dyn Data>,
    ) -> Application {
        Application {
            strategy,
            selector,
            auth,
            data,
            bureou: Bureaucracy::new(),
        }
    }

    pub fn start(&mut self, root_link: Link) -> Result<(), AnyErr> {
        self.data.add(WebNode {
            link: root_link,
            state: CrawlState::Canidate,
        });
        self.work_sync()
    }

    pub fn work_sync(&mut self) -> Result<(), AnyErr> {
        while !self.strategy.end(&self.data) {
            let pass_start_time = std::time::Instant::now();
            let next_nodes = self.strategy.next_nodes(&self.data);

            let mut action_results = Vec::new();
            for (action, link) in next_nodes {
                //async spawn
                let res = 'r: {
                    use Action::*;
                    use CrawlState::*;
                    match action {
                        Explore => {
                            let resp = self.bureou.get_url(&link, false); //await here
                            if resp.is_err() {
                                break 'r CrawlState::Failed;
                            }
                            let body = resp.unwrap();
                            let canidates = self.selector.extract_canidates(&body);
                            break 'r Explored(canidates);
                        }
                        Validate => {
                            let resp = self.bureou.get_url(&link, true); //await here
                            if resp.is_err() {
                                break 'r Failed;
                            }
                            let ct = resp.unwrap();
                            match self.auth.is_valid_from_content_type(&ct) {
                                true => {
                                    break 'r Verified;
                                }
                                false => {
                                    break 'r Rejected;
                                }
                            }
                        }
                    }
                };
                //async merge
                action_results.push(ActionResult::new(link, res));
            }

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
