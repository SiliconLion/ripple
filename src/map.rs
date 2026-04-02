use crate::webutils::*;
use petgraph::dot::{Config, Dot};
use petgraph::matrix_graph::NodeIndex;
use petgraph::prelude::StableDiGraph;
use url::Url;

use std::collections::*;
use std::time::Duration;

// use crate::webutils::have_same_domain;
use crate::webutils::*;

use anyhow::{bail, Context};
type AnyErr = anyhow::Error;

static URL_CHAR_LIMIT: usize = 6000;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum CrawlState {
    Uncrawled,
    // InProgress,
    Crawled,
    Unreachable,
    Forbidden,
}
use CrawlState::*;

//ToDo: now that we are using a StableGraph + hashmap rather than HashGraph can hold a string rather than a fixed buffer if we want
#[derive(Clone)]
pub struct WebNode {
    //page info
    url: [char; URL_CHAR_LIMIT + 1], // +1 to have '\0' at the end in case i need a Cstr
    url_len: usize,

    //graph info
    state: CrawlState,
    depth: u32,

    body: Option<String>,
}

impl WebNode {
    pub fn new(state: CrawlState, url_str: &str) -> WebNode {
        let mut node = WebNode {
            state,
            url: ['\0'; URL_CHAR_LIMIT + 1],
            url_len: 0,
            depth: 0,
            body: None,
        };

        if url_str.len() > URL_CHAR_LIMIT {
            panic!(
                "Cannot create WebNode with Url longer than {} characters",
                URL_CHAR_LIMIT
            )
        }
        for (i, character) in url_str.chars().enumerate() {
            node.url[i] = character;
        }
        node.url_len = url_str.len();
        node
    }
    //should it be "to string" or "as string"? Or string_from_url? Returns a new string...
    pub fn url_to_string(&self) -> String {
        return self.url.iter().collect();
    }
}

impl std::fmt::Debug for WebNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "*")
    }
}

const MAX_DEPTH: u32 = 10;

#[derive(Debug)]
pub struct DomainMap {
    explored: bool,
    domain: String,
    graph: StableDiGraph<WebNode, ()>,
    node_keys: HashMap<String, NodeIndex<u32>>,
    gov: Govenor,
}

impl DomainMap {
    pub fn new(domain: String) -> Result<DomainMap, AnyErr> {
        let explored = false;
        let gov = Govenor::from_link(Url::parse(&domain)?)?;
        let node_keys = HashMap::new();
        let graph = StableDiGraph::<WebNode, ()>::new();

        Ok(DomainMap {
            domain,
            explored,
            gov,
            node_keys,
            graph,
        })
    }

    pub fn keys_to_unexplored_nodes(&self) -> Vec<String> {
        // self.node_keys
        //     .keys
        //     .filter(|(_, node_idx| node.state == Uncrawled)
        //     .collect()
        unimplemented!();
    }

    //does not check to see if that page is already in the graph
    pub fn add_page(&mut self, page: String) {
        let idx = self.graph.add_node(WebNode::new(Uncrawled, &page));
        self.node_keys.insert(page, idx);
    }

    //panics if page does not point to a node in the graph
    pub fn remove_node(&mut self, page: String) -> WebNode {
        let idx = self.node_keys.get(&page).unwrap();
        return self.graph.remove_node(*idx).unwrap();
    }
    pub fn node_at(&mut self, page: String) -> &mut WebNode {
        unimplemented!();
    }
    pub fn add_edge(&mut self, page1: String, page2: String) {
        let idx1 = self.node_keys.get(&page1).unwrap();
        let idx2 = self.node_keys.get(&page2).unwrap();

        self.graph.add_edge(*idx1, *idx2, ());
    }

    pub fn has_edge(&mut self, page1: String, page2: String) -> bool {
        let idx1 = *self.node_keys.get(&page1).unwrap();
        let idx2 = *self.node_keys.get(&page2).unwrap();
        match self.graph.edges_connecting(idx1, idx2).next() {
            None => false,
            Some(_) => true,
        }
    }

    //panics if there is no edge between these
    pub fn remove_edge(&mut self, page1: String, page2: String) {
        let idx1 = *self.node_keys.get(&page1).unwrap();
        let idx2 = *self.node_keys.get(&page2).unwrap();
        let edge_idx = self.graph.find_edge(idx1, idx2).unwrap();
        self.graph.remove_edge(edge_idx);
    }

    //returns Vec of links to html pages outside of this domain
    //does not bounds check to make sure that node_key is valid
    pub fn explore_node(&mut self, node_key: &String) -> Result<Vec<String>, AnyErr> {
        let node_idx = self.node_keys[node_key];
        let node = &self.graph[node_idx];

        let body = match &node.body {
            Some(b) => b.clone(),
            None => self.gov.get_url(&node.url_to_string())?,
        };
        let html_links = self.gov.html_links_from_page_body(body);

        let inner_links: Vec<String> = html_links
            .iter()
            .filter(|link| have_same_domain(&self.domain, link))
            .map(|link| link.clone())
            .collect();
        let outer_links: Vec<String> = html_links
            .iter()
            .filter(|link| !have_same_domain(&self.domain, link))
            .map(|link| link.clone())
            .collect();

        for link in inner_links {
            if self.node_keys.contains_key(&link) {
                let link_idx = self.node_keys[&link];
                self.graph.add_edge(node_idx, link_idx, ());
            } else {
                let link_idx = self.graph.add_node(WebNode::new(Uncrawled, &link));
                self.graph.add_edge(node_idx, link_idx, ());
            }
        }

        return Ok(outer_links);
    }

    pub fn explore_all_nodes(&mut self) -> Vec<(String, Vec<String>)> {
        let keys: Vec<String> = self.node_keys.keys().map(|key| key.clone()).collect();

        let mut all_outer_links: Vec<(String, Vec<String>)> = Vec::with_capacity(keys.len());
        for key in keys {
            let outer_links = self.explore_node(&key).unwrap_or_else(|_| Vec::new());
            all_outer_links.push((key.clone(), outer_links));
        }

        self.explored = true;
        return all_outer_links;
    }
}

#[derive(Clone, Debug)]
pub struct DomainLink {
    domain_from: String,
    page_from: String,
    domain_to: String,
    page_to: String,
}

pub struct WebMap {
    pub graph: StableDiGraph<DomainMap, DomainLink, u32>,
    pub node_keys: HashMap<String, NodeIndex<u32>>,
}

static EXPLORATION_DEPTH: u32 = 3;

impl WebMap {
    pub fn new() -> WebMap {
        let graph = StableDiGraph::new();
        let node_keys = HashMap::new();
        WebMap { graph, node_keys }
    }
    //this doesn't validate the domain or anything like that
    pub fn add_domain(&mut self, domain_name: &String) -> Result<NodeIndex<u32>, AnyErr> {
        let domain_idx = self.graph.add_node(DomainMap::new(domain_name.clone())?);
        self.node_keys.insert(domain_name.clone(), domain_idx);
        return Ok(domain_idx);
    }

    //returns the name of the domain on sucess
    pub fn add_page(&mut self, page: &String) -> Result<String, AnyErr> {
        let url = Url::parse(&page)?;
        let domain_name = match url.domain() {
            Some(d) => d,
            None => {
                bail!("page has no domain")
            }
        };

        let domain_idx = self.add_domain(&From::from(domain_name))?;
        let domain = &mut self.graph[domain_idx];
        domain.add_page(page.clone());
        return Ok(domain_name.to_string());
    }

    pub fn add_links(&mut self, links: Vec<DomainLink>) -> Result<(), AnyErr> {
        //ToDo: Should we be verifying these links? ie, trying to load them or request their heads or something before adding them?
        // For now we will not
        for link in links {
            let domain_from_idx = self.node_keys[&link.domain_from];
            let domain_to_idx = match self.node_keys.get(&link.domain_to) {
                Some(idx) => idx.clone(),
                None => self.add_domain(&link.domain_to)?,
            };
            self.graph
                .add_edge(domain_from_idx, domain_to_idx, link.clone());
            //so now by this point we have the connection between *domains*, but need to add the *pages*
            //within the domains

            let domain_to = &mut self.graph[domain_to_idx];
            if domain_to.node_keys.contains_key(&link.page_to) != true {
                domain_to.add_page(link.page_to);
            } //else do nothing because that page is already in the domain.
              //we don't need to duplicate this for domain_from because the link comes from a page in a domain
        }
        Ok(())
    }

    //ToDo,
    pub fn explore_domains(&mut self, domain_names: Vec<String>) -> Result<(), AnyErr> {
        for domain_name in domain_names {
            let dom_idx = match self.node_keys.get(&domain_name) {
                Some(idx) => idx.clone(),
                None => self.add_domain(&domain_name)?,
            };

            let domain = &mut self.graph[dom_idx]; //valid because we just made sure dom_idx points to something in the graph
            let domain_link_lists = domain.explore_all_nodes();

            let mut all = Vec::with_capacity(domain_link_lists.len() * 5);

            for link_list in domain_link_lists {
                let (page_from, pages_to) = link_list.clone();
                //Todo: handle this unwrap?
                let domain_from = String::from(Url::parse(&page_from)?.domain().unwrap());

                for page_to in &pages_to {
                    let domain_to = String::from(Url::parse(&page_to)?.domain().unwrap());
                    all.push(DomainLink {
                        domain_from: String::from(domain_from.clone()),
                        page_from: String::from(page_from.clone()),
                        domain_to: String::from(domain_to.clone()),
                        page_to: String::from(page_to.clone()),
                    });
                }
            }
            self.add_links(all);
        }
        Ok(())
    }
    pub fn explore_all_domains(&mut self) {
        let domain_names: Vec<String> = self.node_keys.keys().map(ToOwned::to_owned).collect();
        self.explore_domains(domain_names);
    }
}
