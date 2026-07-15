use crate::interface::CrawlState::Canidate;
use crate::interface::CrawlState::Verified;
use crate::interface::*;
use crate::utils::*;
use crate::Link;
use rand::prelude::*;
use select::document::Document;

pub struct ShuffleStrat {
    pub max_at_once: usize,
    pub max_nodes: usize,
    max_poll_frequency: std::time::Duration,
    terminate: bool,
    complete: bool,
    rng: ThreadRng,
}

impl Strategy for ShuffleStrat {
    fn next_nodes(&mut self, data: &Box<dyn Data>) -> Vec<(ActionType, Link)> {
        use crate::interface::ActionType::*;
        use crate::interface::CrawlState::*;

        let mut nodes: Vec<Link> = data.all_nodes();
        if nodes.len() >= self.max_nodes {
            self.terminate = true;
        }

        nodes.shuffle(&mut self.rng);

        let actions: Vec<(ActionType, Link)> = nodes
            .iter()
            .take(self.max_at_once)
            .filter_map(|link| {
                if !self.terminate {
                    match data.get(link).state {
                        Canidate => Some((Validate, link.clone())),
                        Verified => Some((Explore, link.clone())),
                        _ => None,
                    }
                } else {
                    match data.get(link).state {
                        Canidate => Some((Validate, link.clone())),
                        _ => None,
                    }
                }
            })
            .collect();

        if actions.len() == 0 {
            self.complete = true;
        }

        return actions;
    }

    fn end(&mut self, _data: &Box<dyn Data>) -> bool {
        self.complete
    }

    fn max_poll_frequency(&self) -> std::time::Duration {
        self.max_poll_frequency
    }
}

impl ShuffleStrat {
    pub fn new(max_at_once: usize, max_nodes: usize) -> ShuffleStrat {
        ShuffleStrat {
            max_at_once,
            max_nodes,
            max_poll_frequency: std::time::Duration::from_millis(0),
            terminate: false,
            complete: false,
            rng: rand::rng(),
        }
    }
}

pub struct DomainBreadthStrat {
    pub max_at_once: usize,
    pub max_nodes: usize,
    terminate: bool,
    complete: bool,
    rng: ThreadRng,
    last_idx_of_last_round: usize,
}

impl DomainBreadthStrat {
    pub fn new(max_at_once: usize, max_nodes: usize) -> DomainBreadthStrat {
        DomainBreadthStrat {
            max_at_once,
            max_nodes,
            terminate: false,
            complete: false,
            rng: rand::rng(),
            last_idx_of_last_round: 0,
        }
    }
}

impl Strategy for DomainBreadthStrat {
    fn next_nodes(&mut self, data: &Box<dyn Data>) -> Vec<(ActionType, Link)> {
        use crate::interface::ActionType::*;
        println!("starting next nodes");

        if data.total_nodes() >= self.max_nodes {
            self.terminate = true
        }

        let domains = data.domain_names();
        if domains.len() == 0 {
            return Vec::new();
        }

        let mut start: usize = self.last_idx_of_last_round + 1;
        if start >= domains.len() {
            start = 0;
        }

        let mut end: usize = start + self.max_at_once;
        if end >= domains.len() {
            end = domains.len() - 1;
        }

        let mut actions: Vec<(ActionType, Link)> = Vec::with_capacity(domains.len());

        if !self.terminate {
            for domain in &domains[start..=end] {
                let mut nodes = data.get_domain(domain.clone());
                nodes.shuffle(&mut self.rng);

                let mut i = 0;
                while i < nodes.len() {
                    let n = data.get(&nodes[i]);
                    match n.state {
                        Canidate => {
                            actions.push((Validate, nodes[i].clone()));
                            break;
                        }
                        Verified => {
                            actions.push((Explore, nodes[i].clone()));
                            break;
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }
            self.last_idx_of_last_round = end;
        } else {
            self.last_idx_of_last_round = end;

            for node_name in data.all_nodes() {
                let node = data.get(&node_name);
                match node.state {
                    Canidate => {
                        actions.push((Validate, node_name));
                    }
                    _ => {}
                }
            }
        }

        if actions.len() == 0 {
            self.complete = true;
        }

        println!("next nodes complete");
        actions
    }

    fn end(&mut self, _data: &Box<dyn Data>) -> bool {
        self.complete
    }

    fn max_poll_frequency(&self) -> std::time::Duration {
        std::time::Duration::from_millis(0)
    }
}

// #[derive(Clone)]
pub struct HtmlSelector {}
impl Selector for HtmlSelector {
    fn extract_canidates(&self, text: &String) -> Vec<Link> {
        use select::predicate::*;
        // let links: Vec<Link> = Document::from(text.as_str())
        //     .find(Name("body"))
        //     .find(|n| Name("a").or(Name("link")).matches(n))
        //     .iter()
        //     .filter_map(|n| n.attr("href"))
        //     .map(|n| Link::new(&n.to_string()))
        //     .filter_map(|res| res.ok())
        //     .collect();

        // let mut body = Document::from(text.as_str()).find(Name("body"));
        // let mut a = body.find(|node| node.is(Name("a")));

        let links: Vec<Link> = Document::from(text.as_str())
            // .find(Name("body"))
            .find(Name("a").or(Name("link")))
            // .find(|node| node.is(Name("a").or(Name("link"))))
            // .iter()
            .filter_map(|n| n.attr("href"))
            .map(|n| Link::new(&n.to_string()))
            .filter_map(|res| res.ok())
            .collect();

        let mut html_canidates = Vec::with_capacity(links.len());

        for link in links {
            //if there is no extension listed, we do not filter it out
            if let Some(ext) = get_ext(&link.as_string()) {
                if ext != ".html" || ext != ".txt" || ext != ".rtf" || ext != ".xml" {
                    continue;
                }
            }

            html_canidates.push(link)
        }

        html_canidates
    }
}

impl HtmlSelector {
    pub fn new() -> HtmlSelector {
        HtmlSelector {}
    }
}

// #[derive(Clone)]
pub struct HtmlChecker {}
impl Authenticator for HtmlChecker {
    fn is_valid_from_content_type(&self, content_type: &String) -> bool {
        return content_type.contains("html")
            || content_type.contains("HTML")
            || content_type.contains("text");
    }
}

impl HtmlChecker {
    pub fn new() -> HtmlChecker {
        HtmlChecker {}
    }
}
