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

        println!("actions: {actions:?}");

        if actions.len() == 0 {
            self.complete = true;
        }

        return actions;
    }

    fn end(&mut self, data: &Box<dyn Data>) -> bool {
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

        let links: Vec<Link> = Document::from(text.as_str())
            .find(Name("a").or(Name("link")))
            .filter_map(|n| n.attr("href"))
            .map(|n| Link::new(&n.to_string()))
            .filter_map(|res| res.ok())
            .collect();

        // println!("\n\n\nlinks: [");
        // for link in &links {
        //     println!("{link}");
        // }
        // println!("]");

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
}

impl HtmlChecker {
    pub fn new() -> HtmlChecker {
        HtmlChecker {}
    }
}
