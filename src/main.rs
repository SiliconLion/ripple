pub mod error;
pub mod gov;
pub mod link;
pub mod map;
pub mod utils;

use crate::map::{AnyErr, WebMap};

use crate::link::Link;
use petgraph::dot::{Config, Dot};
use std::time::Duration;
use url::Url;

fn main() -> Result<(), AnyErr> {
    let root_page = String::from("https://www.talesofaredclayrambler.com/episodes?year=2017");
    // let root_page = String::from("https://www.goodmorningandgoodnight.com/");
    // let root_page = String::from("https://www.scrapethissite.com/pages/");
    let exploration_depth = 1;

    let mut web = WebMap::new();
    web.add_page(&Link::new(&root_page)?)?;
    for _ in 0..exploration_depth {
        web.explore_all_domains();
    }

    let basic_dot = Dot::with_config(&web.graph, &[Config::EdgeNoLabel, Config::NodeNoLabel]);
    std::fs::write("ripples.dot", format!("{:?}", basic_dot))
        .expect("should be able to write a file");
    // println!("{:?}", basic_dot);
    println!("Complete");

    Ok(())
}
