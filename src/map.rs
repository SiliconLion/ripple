use crate::gov::*;
use petgraph::dot::{Config, Dot};
use petgraph::matrix_graph::NodeIndex;
use petgraph::prelude::StableDiGraph;
use url::Url;

use std::collections::*;

use crate::gov::*;
use crate::link::Link;

use anyhow::{bail, Context};
pub type AnyErr = anyhow::Error;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum CrawlState {
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
    link: Link,
    body: Option<String>,

    //graph info
    state: CrawlState,
    depth: u32,
}

impl WebNode {
    pub fn new(state: CrawlState, link: Link) -> WebNode {
        WebNode {
            state,
            link,
            depth: 0,
            body: None,
        }
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
    domain: String, //has www. stripped but does have TLD
    graph: StableDiGraph<WebNode, ()>,
    node_keys: HashMap<Link, NodeIndex<u32>>, //all these links should have the same domain as self.domain of course
    gov: Govenor,
}

impl DomainMap {
    pub fn new(domain: String) -> Result<DomainMap, AnyErr> {
        let explored = false;
        let gov = Govenor::from_domain(&domain)?;
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

    pub fn get_url_to_domain(&self) -> Url {
        Url::parse(&(String::from("https://") + &self.domain)).unwrap()
    }

    // pub fn keys_to_unexplored_nodes(&self) -> Vec<String> {
    //     // self.node_keys
    //     //     .keys
    //     //     .filter(|(_, node_idx| node.state == Uncrawled)
    //     //     .collect()
    //     unimplemented!();
    // }

    //does not check to see if that page is already in the graph
    pub fn add_page(&mut self, page: &Link) {
        println!("adding page {page:?}");
        let idx = self.graph.add_node(WebNode::new(Uncrawled, page.clone()));
        self.node_keys.insert(page.clone(), idx);
    }

    //panics if page does not point to a node in the graph
    pub fn remove_node(&mut self, page: &Link) -> WebNode {
        let idx = self.node_keys.get(&page).unwrap();
        return self.graph.remove_node(*idx).unwrap();
    }
    pub fn node_at(&mut self, page: &Link) -> &mut WebNode {
        &mut self.graph[self.node_keys[page]]
    }
    pub fn add_edge(&mut self, page1: &Link, page2: &Link) {
        let idx1 = self.node_keys.get(&page1).unwrap();
        let idx2 = self.node_keys.get(&page2).unwrap();

        self.graph.add_edge(*idx1, *idx2, ());
    }

    pub fn has_edge(&mut self, page1: &Link, page2: &Link) -> bool {
        let idx1 = *self.node_keys.get(&page1).unwrap();
        let idx2 = *self.node_keys.get(&page2).unwrap();
        match self.graph.edges_connecting(idx1, idx2).next() {
            None => false,
            Some(_) => true,
        }
    }

    //panics if there is no edge between these
    pub fn remove_edge(&mut self, page1: &Link, page2: &Link) {
        let idx1 = *self.node_keys.get(&page1).unwrap();
        let idx2 = *self.node_keys.get(&page2).unwrap();
        let edge_idx = self.graph.find_edge(idx1, idx2).unwrap();
        self.graph.remove_edge(edge_idx);
    }

    //returns Vec of links to html pages outside of this domain
    //does not bounds check to make sure that node_key is valid
    pub fn explore_node(&mut self, node_key: &Link) -> Result<Vec<Link>, AnyErr> {
        println!("exploring node: {node_key:?}");
        let node_idx = self.node_keys[node_key];
        let node = &mut self.graph[node_idx];

        println!("in explore_node");

        let body = match &node.body {
            Some(b) => b.clone(),
            None => self.gov.get_url(&node.link.as_string(), false)?,
        };
        println!("got node body");

        let html_links = self.gov.html_links_from_page_body(body);

        let self_dom = self.domain.clone(); //just doing this to avoid a borrowing conflict
        let inner_links: Vec<Link> = html_links
            .iter()
            .filter(|link| self_dom == link.domain)
            .map(|link| link.clone())
            .collect();
        let outer_links: Vec<Link> = html_links
            .iter()
            .filter(|link| self_dom != link.domain)
            .map(|link| link.clone())
            .collect();

        println!("inner_links: {inner_links:?}");
        println!("outer_links: {outer_links:?}");

        node.state = CrawlState::Crawled;

        for link in inner_links {
            if self.node_keys.contains_key(&link) {
                let link_idx = self.node_keys[&link];
                self.graph.add_edge(node_idx, link_idx, ());
            } else {
                let link_idx = self.graph.add_node(WebNode::new(Uncrawled, link));
                self.graph.add_edge(node_idx, link_idx, ());
            }
        }

        return Ok(outer_links);
    }

    pub fn explore_all_nodes(&mut self) -> Vec<(Link, Vec<Link>)> {
        let keys: Vec<Link> = self.node_keys.keys().map(|key| key.clone()).collect();

        let mut all_outer_links: Vec<(Link, Vec<Link>)> = Vec::with_capacity(keys.len());
        for key in keys {
            let outer_links = match self.explore_node(&key) {
                Ok(l) => l,
                Err(e) => {
                    println!("{e}");
                    Vec::new()
                }
            };
            all_outer_links.push((key.clone(), outer_links));
        }

        self.explored = true;
        return all_outer_links;
    }
}

#[derive(Clone, Debug)]
pub struct DomainEdge {
    from: Link,
    to: Link,
}

impl DomainEdge {
    pub fn new(from: Link, to: Link) -> DomainEdge {
        DomainEdge { from, to }
    }
}

#[derive(Debug)]
pub struct WebMap {
    pub graph: StableDiGraph<DomainMap, DomainEdge, u32>,
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
        println!("domain_name: {domain_name}");
        let domain_idx = self.graph.add_node(DomainMap::new(domain_name.clone())?);
        self.node_keys.insert(domain_name.clone(), domain_idx);
        return Ok(domain_idx);
    }

    pub fn get_domain_or_add(&mut self, domain_name: &String) -> Result<&mut DomainMap, AnyErr> {
        let idx = match self.node_keys.contains_key(domain_name) {
            true => self.node_keys[domain_name],
            false => self.add_domain(&domain_name)?,
        };
        return Ok(&mut self.graph[idx]);
    }

    //returns the name of the domain on sucess
    pub fn add_page(&mut self, page: &Link) -> Result<String, AnyErr> {
        let domain = self.get_domain_or_add(&page.domain)?;
        domain.add_page(page);
        Ok(page.domain.clone())
    }

    pub fn add_edge(&mut self, from: Link, to: Link) -> Result<(), AnyErr> {
        let domain_from_idx = self.node_keys[&from.domain];
        let domain_to_idx = match self.node_keys.get(&to.domain) {
            Some(idx) => idx.clone(),
            None => self.add_domain(&to.domain)?,
        };
        self.graph.add_edge(
            domain_from_idx,
            domain_to_idx,
            DomainEdge::new(from.clone(), to.clone()),
        );
        //so now by this point we have the edge between *domains*, but need to add the *pages*
        //within the domains. We assume the page already exists in the "from" domain map, so we only need to
        // make sure the page exists in the domain "to"

        let domain_to = &mut self.graph[domain_to_idx];
        if domain_to.node_keys.contains_key(&to) != true {
            domain_to.add_page(&to);
        } //else do nothing because that page is already in the domain.
          //we don't need to duplicate this for domain_from because the link comes from a page in a domain
        Ok(())
    }

    //panics if one of the domain names does not correspond to a domain in the WebMap
    pub fn explore_domains(&mut self, domain_names: Vec<String>) {
        for domain_name in domain_names {
            println!("exploring domain: {domain_name}");
            let dom_idx = self.node_keys.get(&domain_name).unwrap();

            let domain = &mut self.graph[*dom_idx];
            let domain_outside_links = domain.explore_all_nodes();

            for link_list in domain_outside_links {
                let (page_from, pages_to) = link_list.clone();

                for page_to in &pages_to {
                    let res = self.add_edge(page_from.clone(), page_to.clone());
                    if res.is_err() {
                        println!("Error in explore domains. {:?}", res);
                    }
                }
            }
        }
    }
    pub fn explore_all_domains(&mut self) {
        let domain_names: Vec<String> = self.node_keys.keys().map(ToOwned::to_owned).collect();
        self.explore_domains(domain_names);
    }
}
