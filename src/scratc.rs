use crate::webutils::*;
use petgraph::dot::{Config, Dot};
use petgraph::matrix_graph::NodeIndex;
use petgraph::prelude::StableDiGraph;
node_keys: HashMap<String, <usize>>,

use std::collections::*;
use std::time::Duration;

// struct InnerLink {
//     from: String,
//     to: String,
// }

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum CrawlState {
    Uncrawled,
    // InProgress,
    Crawled,
    Unreachable,
    Forbidden
}
use CrawlState::*;

#[derive(Clone, Copy, PartialOrd, Ord)]
struct WebNode {
    //page info
    url: [char; URL_CHAR_LIMIT + 1], // +1 to have '\0' at the end in case i need a Cstr
    url_len: usize,

    //graph info
    state: CrawlState,
    depth: u32,

}

impl WebNode {
    fn new(state: CrawlState, url_str: &str) -> WebNode {
        let mut node = WebNode {
            state,
            url: ['\0'; URL_CHAR_LIMIT + 1],
            url_len: 0,
            depth: 0
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
    fn url_to_string(&self) -> String {
        return self.url.iter().collect();
    }

    async fn get_html_links(
        node: &WebNode,
        client: reqwest::Client,
        blacklist: &Vec<String>,
    ) -> Vec<String> {
        let page_body_res = fetch_url(&client, node.url_to_string().as_str()).await;
        let page_body = match page_body_res {
            Ok(body) => body,
            Err(_) => {
                // println!("cannot fetch {:?}", node.url_to_string());
                return Vec::new(); //could propgate the error but empty vec is the same thing basically
            }
        };

        let root_url = match reqwest::Url::parse(&node.url_to_string()) {
            Err(_) => {
                println!("wtf? cant parse a url that is in a web node? This should have been ");
                panic!()
            }
            Ok(v) => v,
        };

        let links: Vec<String> = get_page_links(&page_body)
            .into_iter()
            .filter(|link| !is_in_domain_list(link, blacklist))
            .map(|link| normalize_url(&link, Some(root_url.clone())))
            .filter_map(|norm_res: Result<String, ParseError>| norm_res.ok())
            .collect();
        if links.len() == 0 {
            return Vec::new();
        }

        let mut html_links = Vec::with_capacity(links.len());

        let (tx, mut rx) = mpsc::channel(links.len());

        for link in links {
            let txx = tx.clone();
            let client_x = client.clone();
            tokio::spawn(async move {
                let ret = get_link_head(&link, client_x).await;
                let mut head;
                match ret {
                    Err(e) => {
                        // println!("Cannot get link head. \nError: {}\nLink: {}", e, link);
                        drop(txx); //would be called automattically, but because it is part of the control flow, i am making it explicit here.
                        return;
                    }
                    Ok(v) => {
                        head = v;
                    }
                }

                match link_is_html_from_head(head) {
                    true => {
                        if let Err(_) = txx.send(link.clone()).await {
                            println!("Cannot send link to reciver. Will panic. Link: {}", link);
                            println!("txx.is_closed = {}", txx.is_closed());
                            panic!();
                        }
                    }
                    false => {}
                };
                drop(txx); //would be called automattically, but because it is part of the control flow, i am making it explicit here.
            });
        }

        //This is what we use to spawn all the transmitters, but if we dont drop this one,
        //the reciver will never finish either.
        drop(tx);

        while let Some(html_link) = rx.recv().await {
            // println!("just added an html link: {}", &html_link);
            html_links.push(html_link);
        }
        // println!("html links: {:?}", html_links);
        return html_links;
    }


    fn get_html_links_blocking(
        node: &WebNode,
        gov: Govenor
    ) -> Vec<String>
    {
        let page_body_res = fetch_url_sync(&client, node.url_to_string().as_str());
        let page_body = match page_body_res {
            Ok(body) => body,
            Err(_) => {
                // println!("cannot fetch {:?}", node.url_to_string());
                return Vec::new(); //could propgate the error but empty vec is the same thing basically
            }
        };

        let root_url = match reqwest::Url::parse(&node.url_to_string()) {
            Err(_) => {
                println!("wtf? cant parse a url that is in a web node? This should have been ");
                panic!()
            }
            Ok(v) => v,
        };

        let links: Vec<String> = get_page_links(&page_body)
            .into_iter()
            .filter(|link| !is_in_domain_list(link, blacklist))
            .map(|link| normalize_url(&link, Some(root_url.clone())))
            .filter_map(|norm_res: Result<String, ParseError>| norm_res.ok())
            .collect();
        if links.len() == 0 {
            return Vec::new(); //no links
        }

        let mut html_links = Vec::with_capacity(links.len());

        for link in links {
            let mut resp = request_head_sync(&link, client_x);

            if !resp.status().is_sucess() {

            }


            let mut head;
            match ret {
                Err(e) => {
                    println!("Cannot get link head. \nError: {}\nLink: {}", e, link);
                    return;
                }
                Ok(v) => {
                    head = v;
                }
            }

            match link_is_html_from_head(head) {
                true => {
                    if let Err(_) = txx.send(link.clone()).await {
                        println!("Cannot send link to reciver. Will panic. Link: {}", link);
                        println!("txx.is_closed = {}", txx.is_closed());
                        panic!();
                    }
                }
                false => {}
            };
        }

        //This is what we use to spawn all the transmitters, but if we dont drop this one,
        //the reciver will never finish either.
        drop(tx);

        while let Some(html_link) = rx.recv().await {
            // println!("just added an html link: {}", &html_link);
            html_links.push(html_link);
        }
        // println!("html links: {:?}", html_links);
        return html_links;
    }
}


impl std::fmt::Debug for WebNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // if self.url_len < 40 {
        //     write!(f, "{}", self.url_to_string())
        // } else {
        //     write!(f, "{}...", &self.url_to_string()[..150])
        // }

        write!(f, "0")
    }
}

static MAX_DEPTH: u32 = 10;

struct DomainMap {
    explored: bool,
    domain: String,
    graph: StableDiGraph<WebNode, ()>,
    node_keys: HashMap<String, NodeIndex<u32>>,
    gov: Govenor
}

impl DomainMap {
    fn new(name: &String) -> DomainMap {}
    fn keys_to_unexplored_nodes(&self) -> Vec<String> {
        self.node_keys.iter().filter(|node| node.state == Uncrawled).collect()
    }

    //returns Vec of links to html pages outside of this domain
    fn explore_node(&mut self, node_key: String) -> Vec<String> {
        let node = self.map.get(node_key);
        let html_links = node.get_html_links();
        let inner_links = html_links
            .iter()
            .filter(|link| is_in_domain(&self.domain, link));
        let outer_links = html_links
            .iter()
            .filter(|link| !is_in_domain(&self.domain, link));

        for link in inner_links {
            if self.map.contains_key(link) {
                self.map.add_edge(node, link, ());
            } else {
                let link_node = WebNode::new(UnCrawled, &link);
                self.map.insert(link_node);
                self.map.add_edge(node, link, ());
            }
        }

        return Ok(outer_links);
    }

    fn explore_all_nodes(&mut self) -> Vec<(String, Vec<String>)> {
        let keys = self.map.keys();

        let all_outer_links: Vec<(String, Vec<String>)> = Vec::with_capacity(keys.len());
        for key in keys {
            let outer_links = self.explore_node(key);
            all_outer_links.push((key, outer_links));
        }
        self.explored = true;
        return all_outer_links;
    }

    // fn explore_depth_first(&mut self) -> Vec<(String, Vec<String>)> {

    //     let link
    // }
}

#[derive(Clone)]
struct DomainLink {
    page_from: String,
    page_to: String,
}

struct WebMap {
    graph: StableDiGraph<DomainMap, DomainLink, u32>,
    node_keys: HashMap<String, NodeIndex<u32>>,
}

enum WebErr {}

static EXPLORATION_DEPTH: u32 = 6;

impl WebMap {
    pub fn new() -> WebMap {
        unimplimented!()
    }
    pub fn add_domain(&mut self, domain_name: String) -> NodeIndex<u32> {
        let domain_idx = self.graph.add_node(DomainMap::new(&domain_name));
        self.node_keys.insert(domain_name, domain_idx);
        return domain_idx;
    }
    pub fn add_links(&mut self, links: Vec<DomainLink>) -> Result<(), WebErr> {
        for link in links {
            let page_from_idx = match self.node_keys.get(&link.page_from) {
                Some(idx) => idx,
                None => self.add_domain(link.page_to.clone())
            };
            let page_to_idx = match self.node_keys.get(&link.page_to) {
                Some(idx) => idx,
                None => self.add_domain(link.page_to.clone())
            };
            self.graph.add_edge(page_from_idx, page_to_idx, link);
        }

        unimplemented!()
    }

    pub fn explore_domains(&mut self, domain_names: Vec<String>) {
        for domain_name in domain_names {
            let dom_idx = match self.node_keys.get(domain_name) {
                Some(idx) => idx,
                None => self.add_domain(domain_name)
            }

            let domain = &mut self.graph[dom_idx].unwrap(); //valid because we just made sure dom_idx points to something in the graph
            let domain_link_lists = domain.explore_all_nodes();

            let all = Vec::with_capacity(domain_link_lists.len() * 5);

            for link_list in domain_link_lists {
                let (page_from, pages_to) = link_list;
                for page in pages_to {
                    all.push(DomainLink{page_from, page});
                }
            }
            self.add_links(all);
        }
    }
    pub fn explore_all_domains(&mut self) {
        let domain_names: Vec<String> = self.node_keys.iter().collect();
        self.explore_domains(domain_names);
    }
}

// pub struct HashGraph<N, E, H> {
//     map: HashMap<H, usize>,
//     graph: StableDiGraph<N, E>
// }

// impl HashGraph<N, E, H> {
//     pub fn add_node
// }
