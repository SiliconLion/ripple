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

pub mod scratc;
pub mod webutils;

use std::ffi::CStr;
use std::fmt::Debug;
use std::future::Future;
use std::time::Duration;

use petgraph::dot::{Config, Dot};
use petgraph::graphmap::DiGraphMap;

// use owo_colors::OwoColorize;

use tokio::sync::mpsc;
use tokio::time::timeout;
// use tokio_stream::StreamExt;

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
        "wikipedia.com",
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

    // let root_url = "https://www.talesofaredclayrambler.com/episodes?year=2017";
    // let root_url = "https://www.goodmorningandgoodnight.com/";
    let root_url = "https://www.scrapethissite.com/pages/";

    let client = reqwest::Client::new();

    let mut g: DiGraphMap<WebNode, ()> = DiGraphMap::new();
    let mut cur_uncrawled = vec![g.add_node(WebNode::new(Uncrawled, root_url))];

    let mut depth = 0;
    while cur_uncrawled.len() != 0 && depth < 10 {
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
                    match timeout(Duration::from_secs(20), fut_html_links).await {
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
