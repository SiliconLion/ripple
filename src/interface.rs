use crate::gov::*;
use crate::interface::CrawlState::Failed;
use crate::link::*;
use crate::utils::*;

#[derive(Clone, Copy, Debug)]
pub enum ActionType {
    Explore,
    Validate,
}

//ToDo: Turn this all into a state machine or something. Can be cleaned up
// #[derive(Clone)]
pub struct ActionIntermediary1 {
    pub link: Link,
    pub action: ActionType,
    pub reply: Reply,
}

// #[derive(Clone)]
pub struct ActionIntermediary2 {
    pub link: Link,
    pub action: ActionType,
    pub resp: Result<String, AnyErr>,
}

#[derive(Clone)]
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
    fn next_nodes(&mut self, data: &Box<dyn Data>) -> Vec<(ActionType, Link)>;
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
        use ActionType::*;
        use CrawlState::*;

        while !self.strategy.end(&self.data) {
            let pass_start_time = std::time::Instant::now();
            let next_nodes = self.strategy.next_nodes(&self.data);

            let mut intermediaries1 = Vec::new();
            let mut intermediaries2 = Vec::new();
            let mut action_results = Vec::new();

            for (action, link) in next_nodes {
                let reply = match action {
                    Explore => self.bureou.request(&link, false)?,
                    Validate => self.bureou.request(&link, true)?,
                };
                let ai = ActionIntermediary1 {
                    link,
                    action,
                    reply,
                };
                intermediaries1.push(ai);
            }

            while intermediaries1.len() > 0 {
                let mut to_remove = Vec::new();

                for (i, ai) in intermediaries1.iter().enumerate() {
                    match ai.reply.reciver.try_recv() {
                        Err(_) => {
                            continue;
                        }
                        Ok(resp) => {
                            to_remove.push(i);
                            intermediaries2.push(ActionIntermediary2 {
                                link: ai.link.clone(),
                                action: ai.action.clone(),
                                resp,
                            })
                        }
                    }
                }

                to_remove.sort();
                for &index in to_remove.iter().rev() {
                    intermediaries1.remove(index);
                }
            }

            for ai in intermediaries2 {
                let state = {
                    if ai.resp.is_err() {
                        Failed
                    } else {
                        match ai.action {
                            Explore => {
                                let body = ai.resp?;
                                let canidates = self.selector.extract_canidates(&body);
                                Explored(canidates)
                            }
                            Validate => {
                                let ct = ai.resp?;
                                match self.auth.is_valid_from_content_type(&ct) {
                                    true => Verified,
                                    false => Rejected,
                                }
                            }
                        }
                    }
                };
                action_results.push(ActionResult {
                    link: ai.link,
                    state,
                });
            }

            for ar in action_results {
                self.data.update(ar);
            }

            //ToDo: make this a callback within data or something.
            let dot = self.data.represent();
            std::fs::write("ripples.dot", format!("{dot}"))
                .expect("should be able to write a file");

            let pass_end_time = std::time::Instant::now();

            let delta = pass_end_time - pass_start_time;
            if delta < self.strategy.max_poll_frequency() {
                std::thread::sleep(self.strategy.max_poll_frequency() - delta);
            }
        }

        return Ok(());
    }
}
