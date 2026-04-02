pub mod error;
pub mod map;
pub mod webutils;

use crate::map::WebMap;

use petgraph::dot::{Config, Dot};
use std::time::Duration;
use url::Url;

fn main() {
    let root_url = Url::parse("https://www.talesofaredclayrambler.com/episodes?year=2017").unwrap();
    // let root_url = Url::parse("https://www.goodmorningandgoodnight.com/").unwrap();
    // let root_url = Url::parse("https://www.scrapethissite.com/pages/").unwrap();
    let exploration_depth = 3;

    let mut web = WebMap::new();
    web.add_page(&String::from(root_url.as_str()));

    for _ in 0..exploration_depth {
        web.explore_all_domains();
    }

    let basic_dot = Dot::new(&web.graph);
    std::fs::write("ripples.dot", format!("{:?}", basic_dot))
        .expect("should be able to write a file");
    // println!("{:?}", basic_dot);
    println!("Complete");
}
