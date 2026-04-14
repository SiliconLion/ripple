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
    // let root_page = String::from("https://www.talesofaredclayrambler.com/episodes?year=2017");
    // let root_page = String::from("https://www.goodmorningandgoodnight.com/");
    // let root_page = String::from("https://www.scrapethissite.com/pages/");
    // let root_page = String::from("https://bored.com/");
    let root_page = String::from("https://www.math3ma.com/");
    let exploration_depth = 2;

    let mut web = WebMap::new();
    web.add_page(&Link::new(&root_page)?)?;
    for _ in 0..exploration_depth {
        web.explore_all_domains();
    }

    // let dot = Dot::with_config(&web.graph, &[Config::EdgeNoLabel, Config::NodeNoLabel]);

    let dot = Dot::with_attr_getters(
        &web.graph,
        &[Config::EdgeNoLabel, Config::NodeNoLabel],
        &|_, edgeref| format!("color = blue, penwidth = {}", edgeref.weight().len()).to_string(),
        &|_, (_, dom_map)| format!("label = \"{}\"", dom_map.domain.clone()),
    );

    std::fs::write("ripples.dot", format!("{:?}", dot)).expect("should be able to write a file");

    println!("Complete");
    Ok(())
}
