pub mod error;
pub mod gov;
pub mod hashdata;
pub mod interface;
pub mod link;
pub mod simple_impl;
// pub mod map;
pub mod utils;

use crate::error::*;
use crate::hashdata::HashData;
use crate::interface::Application;
use crate::link::Link;
use crate::simple_impl::{HtmlChecker, HtmlSelector, ShuffleStrat};
// use petgraph::dot::{Config, Dot};
// use std::time::Duration;
// use url::Url;

fn main() -> Result<(), AnyErr> {
    let root_page = String::from("https://www.talesofaredclayrambler.com/episodes?year=2017");
    // let root_page = String::from("https://www.goodmorningandgoodnight.com/");
    // let root_page = String::from("https://www.scrapethissite.com/pages/");
    // let root_page = String::from("https://bored.com/");
    // let root_page = String::from("https://www.math3ma.com/");
    // let root_page = String::from(
    //     "https://www.reddit.com/r/Blogging/comments/1josfud/your_favourite_blog_in_2025/",
    // );
    // let root_page =
    //     String::from("https://www.theintrinsicperspective.com/p/writing-for-outlets-isnt-worth-it");
    // let root_page = String::from(
    //     "https://lithub.com/the-joy-and-privilege-of-growing-up-in-an-indie-bookstore/",
    // );

    // let mut web = WebMap::new();
    // web.add_page(&Link::new(&root_page)?)?;
    // for _ in 0..exploration_depth {
    //     web.explore_all_domains();
    // }

    let mut app = Application::new(
        Box::new(ShuffleStrat::new(15, 600)),
        Box::new(HtmlSelector::new()),
        Box::new(HtmlChecker::new()),
        Box::new(HashData::new()),
    );

    app.start(Link::new(&root_page)?)?;

    // let dot = Dot::with_attr_getters(
    //     &web.graph,
    //     &[Config::EdgeNoLabel, Config::NodeNoLabel],
    //     &|_, edgeref| format!("color = blue, penwidth = {}", edgeref.weight().len()).to_string(),
    //     &|_, (_, dom_map)| format!("label = \"{}\"", dom_map.domain.clone()),
    // );

    // std::fs::write("ripples.dot", format!("{:?}", dot)).expect("should be able to write a file");
    let dot = app.data.represent();
    std::fs::write("ripples.dot", format!("{dot}")).expect("should be able to write a file");

    println!("Complete");
    Ok(())
}
