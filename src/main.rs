// two modes: depth mode and breadth mode
//
// breadth mode:
//- adds origin url as "uncrawled node of graph
//- while graph.uncrawled_nodes.len != 0
//- let source_node = graph.uncrawled_nodes.first()
//- crawls entire domain of source_node Compiles list of all links.
//- every link to outside source is added as
// a new node in the graph with a connection from the source node UNLESS
// that site is already in the graph, then there is just a connection added between them
// source_node removed from uncrawled
//

use std::ffi::CStr;
use std::fmt::Debug;

use petgraph::dot::{Config, Dot};
use petgraph::graphmap::DiGraphMap;
use reqwest::IntoUrl;
use reqwest::Url;
use select::document::Document;
use select::predicate::Name;
use select::predicate::Predicate;

use owo_colors::OwoColorize;

//ToDo: Fix: Better error handling
// promis i will tomorrow
fn fetch_url(client: &reqwest::blocking::Client, url: &str) -> Result<String, reqwest::Error> {
    let mut headers = reqwest::header::HeaderMap::new();

    // headers.insert("authorization", "<authorization>".parse().unwrap());
    headers.insert("user-agent", "'Mozilla/5.0".parse().unwrap());

    let mut res = client.get(url).headers(headers).send()?;
    // println!("Status for {}: {}", url, res.status());

    let mut body = res.text()?;
    Ok(body)
}

//links to webpages. ie, not scripts, images, resources, etc
fn get_page_links(node: &WebNode, client: &reqwest::blocking::Client) -> Vec<String> {
    //ToDo: Fix: dont just unwrap.
    // I promise I will do better error handling in the morning
    let response = fetch_url(client, &node.url_to_string());

    if response.is_err() {
        return Vec::new();
    }
    let body = response.unwrap();

    // println!("URL {} links to :", node.url_to_string());
    let links = Document::from(body.as_str())
        .find(Name("a").or(Name("link")))
        .filter_map(|n| n.attr("href"))
        .map(|n| n.to_string())
        .collect();
    for link in &links {
        // println!("{}", &link);
    }
    return links;
}

fn link_is_html(link: &String) -> bool {}

fn is_in_domain_list(url: &String, list: &Vec<&str>) -> bool {
    use reqwest::Error;

    match Url::parse(url) {
        Ok(parsed_url) => {
            if let Some(domain) = parsed_url.domain() {
                list.contains(&domain)
            } else {
                // url has root domain, ie, the domain is "/".
                //we will assume for now that is not in any list.
                false
            }
        }
        // Err(e) => {
        //     println!("Error, url cannot be parsed. {}", e);
        //     panic!()
        // }
        Err(e) => false, // implicitly, the url is not in the list
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum CrawlState {
    Uncrawled,
    InProgress,
    Complete,
    Unreachable,
}
use CrawlState::*;

#[derive(Clone, Copy, PartialOrd, Ord)]
struct WebNode {
    state: CrawlState,
    url: [char; URL_CHAR_LIMIT + 1], // +1 to have '\0' at the end in case i need a Cstr
    url_len: usize,
}

impl WebNode {
    fn new(state: CrawlState, url_str: &str) -> WebNode {
        let mut node = WebNode {
            state,
            url: ['\0'; URL_CHAR_LIMIT + 1],
            url_len: 0,
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
}

impl std::hash::Hash for WebNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.url.hash(state);
    }
}

impl PartialEq for WebNode {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

impl Eq for WebNode {
    //surprisingly this is a trait without methods. its an assertion on top of ParialEq
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

static URL_CHAR_LIMIT: usize = 6000;

fn main() {
    let stubs = vec![
        "facebook.com",
        "youtube.com",
        "instagram.com",
        "x.com",
        "stackoverflow.com",
        "adobe.com",
        "patreon.com",
    ];
    let blacklist = vec![
        "use.typekit.net",
        "cdn.cookielaw.org",
        "assets.adobedtm.com",
    ];
    // let root_url = "https://www.talesofaredclayrambler.com/episodes?year=2017";
    let root_url = "https://www.goodmorningandgoodnight.com/";

    let client = reqwest::blocking::Client::new();

    let mut g: DiGraphMap<WebNode, ()> = DiGraphMap::new();
    let mut cur_uncrawled = vec![g.add_node(WebNode::new(Uncrawled, root_url))];

    let mut depth = 0;
    while cur_uncrawled.len() != 0 && depth < 2 {
        let mut new_uncrawled = Vec::with_capacity(cur_uncrawled.len() * 4);

        for mut node in &mut cur_uncrawled {
            if is_in_domain_list(&node.url_to_string(), &stubs) {
                node.state = Complete;
                continue;
            }

            node.state = InProgress;

            let page_links: Vec<String> = get_page_links(&node, &client)
                .into_iter()
                .filter(|link| !is_in_domain_list(link, &blacklist))
                .collect();
            for page in page_links {
                let pg = WebNode::new(Uncrawled, &page);
                if g.contains_node(pg) {
                    g.add_edge(*node, pg, ());
                } else {
                    new_uncrawled.push(g.add_node(WebNode::new(Uncrawled, &page)));
                    g.add_edge(*node, pg, ());
                }
            }
            node.state = Complete;
        }

        cur_uncrawled = new_uncrawled;
        depth += 1;
    }

    let basic_dot = Dot::new(&g);
    std::fs::write("ripples.dot", format!("{:?}", basic_dot))
        .expect("should be able to write a file");
    // println!("{:?}", basic_dot);
    println!("Complete");
}
