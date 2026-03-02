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
use std::future::Future;
use std::time::Duration;

use petgraph::dot::{Config, Dot};
use petgraph::graphmap::DiGraphMap;

use reqwest::blocking::Request;
use reqwest::IntoUrl;

use url::{ParseError, Url};

use select::document::Document;
use select::predicate::Name;
use select::predicate::Predicate;

// use owo_colors::OwoColorize;

use tokio::sync::mpsc;
use tokio::time::timeout;
// use tokio_stream::StreamExt;

async fn fetch_url(client: &reqwest::Client, url: &str) -> Result<String, reqwest::Error> {
    let mut headers = reqwest::header::HeaderMap::new();
    //
    //ToDo: is this unwrap okay?
    headers.insert("user-agent", "'Mozilla/5.0".parse().unwrap());
    let res = client.get(url).headers(headers).send().await?;
    let body = res.text().await?;
    Ok(body)
}

// //links to webpages. ie, not scripts, images, resources, etc
// async fn get_page_body(node: &WebNode, client: &reqwest::Client) -> Result<String, reqwest::Error> {
//     let response = fetch_url(client, &node.url_to_string()).await?;
// }

fn get_page_links(body: &str) -> Vec<String> {
    let links = Document::from(body)
        .find(Name("a").or(Name("link")))
        .filter_map(|n| n.attr("href"))
        .map(|n| n.to_string())
        .collect();
    return links;
}

async fn get_link_head(
    link: &String,
    client: reqwest::Client,
) -> Result<reqwest::header::HeaderMap, reqwest::Error> {
    let mut our_headers = reqwest::header::HeaderMap::new();
    our_headers.insert("user-agent", "'Mozilla/5.0".parse().unwrap());

    let ret = client.head(link).headers(our_headers).send().await?;
    Ok(ret.headers().clone())
}

fn link_is_html_from_head(head: reqwest::header::HeaderMap) -> bool {
    if let Some(ct) = head.get("Content-type") {
        let ct_str = ct.to_str().unwrap_or("");
        return ct_str.contains("html") || ct_str.contains("HTML") || ct_str.contains("text/plain");
    } else {
        return false;
    }
}

//todo: I know there is a lot of going back and forth between strings and Url's thats not strictly necessary.
fn normalize_url(url: &String, root: Option<reqwest::Url>) -> Result<String, ParseError> {
    match Url::parse(url) {
        Err(e) => {
            if url.starts_with("/") {
                match root {
                    Some(root_contents) => {
                        let joined = root_contents.join(url)?;
                        normalize_url(&String::from(joined), None)
                    }
                    None => Err(e),
                }
            } else {
                return Err(e);
            }
        }
        Ok(parsed_url) => Ok(String::from(parsed_url.as_str())),
    }
}

fn is_in_domain_list(url: &String, list: &Vec<String>) -> bool {
    match reqwest::Url::parse(url) {
        Ok(parsed_url) => {
            if let Some(domain) = parsed_url.domain() {
                for item in list {
                    if item.contains(&domain) {
                        return true;
                    }
                }
                return false;
            } else {
                // url has root domain, ie, the domain is "/".
                //we will assume for now that is not in any list.
                false
            }
        }
        Err(_) => false, // implicitly, the url is not in the list
    }
}

async fn get_html_links_from_node(
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

#[tokio::main]
async fn main() {
    //these lists should perhaps be Arc<Vec<String>>
    let stubs: Vec<String> = vec![
        "facebook.com",
        "youtube.com",
        "instagram.com",
        "x.com",
        "twitter.com",
        "stackoverflow.com",
        "adobe.com",
        "patreon.com",
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect();

    let blacklist: Vec<String> = vec![
        "use.typekit.net",
        "cdn.cookielaw.org",
        "assets.adobedtm.com",
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect();

    let root_url = "https://www.talesofaredclayrambler.com/episodes?year=2017";
    // let root_url = "https://www.goodmorningandgoodnight.com/";

    let client = reqwest::Client::new();

    let mut g: DiGraphMap<WebNode, ()> = DiGraphMap::new();
    let mut cur_uncrawled = vec![g.add_node(WebNode::new(Uncrawled, root_url))];

    let mut depth = 0;
    while cur_uncrawled.len() != 0 && depth < 6 {
        let mut new_uncrawled = Vec::with_capacity(cur_uncrawled.len() * 4);

        let (tx, mut rx) = mpsc::channel(cur_uncrawled.len());

        for node in cur_uncrawled {
            if is_in_domain_list(&node.url_to_string(), &stubs) {
                // node.state = Complete;
                println!(
                    "skipping this node because it is a stub! Url: {}",
                    node.url_to_string()
                );
                continue;
            }
            // node.state = InProgress;

            let client_x = client.clone();
            let blacklist_x = blacklist.clone();
            let node_x = node.clone();

            let txx = tx.clone();
            tokio::spawn(async move {
                let fut_html_links = get_html_links_from_node(&node_x, client_x, &blacklist_x);

                let html_links: Vec<String> =
                    match timeout(Duration::from_secs(5), fut_html_links).await {
                        Err(e) => {
                            println!("did not receive value within 5s. Err: {}", e);
                            Vec::new()
                        }
                        Ok(v) => v,
                    };

                if html_links.len() == 0 {
                    println!("no html links from this page");
                    return;
                }

                if let Err(e) = txx.send((node.clone(), html_links)).await {
                    println!("error trying to send html links to reciver. Error: {}", e);
                    //should I drop txx here?
                    panic!();
                }
                drop(txx); // would be done automattically
                           // but im making it clear this is part of the control flow
            });
        }

        //This is what we use to spawn all the transmitters, but if we dont drop this one,
        //the reciver will never finish either.
        drop(tx);

        while let Some((node, html_links)) = rx.recv().await {
            for link in html_links {
                let pg = WebNode::new(Uncrawled, &link);
                if g.contains_node(pg) {
                    g.add_edge(node, pg, ());
                } else {
                    new_uncrawled.push(g.add_node(WebNode::new(Uncrawled, &link)));
                    g.add_edge(node, pg, ());
                }
                // println!("node added");
            }
        }

        println!("done adding nodes for level {}", depth);

        // node.state = Complete;

        cur_uncrawled = new_uncrawled;
        depth += 1;
    }

    let basic_dot = Dot::new(&g);
    std::fs::write("ripples.dot", format!("{:?}", basic_dot))
        .expect("should be able to write a file");
    // println!("{:?}", basic_dot);
    println!("Complete");
}
