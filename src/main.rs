pub mod gov;
pub mod hashdata;
pub mod interface;
pub mod link;
pub mod simple_impl;
pub mod utils;

use crate::hashdata::HashData;
use crate::interface::Application;
use crate::link::Link;
use crate::simple_impl::{HtmlChecker, HtmlSelector, ShuffleStrat};
use crate::utils::*;

use std::env;

fn main() -> Result<(), AnyErr> {
    let args: Vec<String> = env::args().collect();
    // first arg is always name of program, so len_of_our_args + 1
    if args.len() < 3 {
        bail!("expected at least two arguments: target_node_count and root_page");
    }

    let target_node_count = match args[1].parse::<usize>() {
        Ok(count) => count,
        Err(_) => bail!("first argument is expected to be a positive number"),
    };
    if target_node_count <= 0 {
        bail!("first argument is expected to be a positive number");
    }

    let root_link = match Link::new(&args[2]) {
        Ok(link) => link,
        Err(_) => bail!("Second argument must be a valid URL (including schema)"),
    };

    // let root_page = String::from("https://www.talesofaredclayrambler.com/episodes?year=2017");
    // let root_page = String::from("https://www.goodmorningandgoodnight.com/");
    // let root_page = String::from("https://www.scrapethissite.com/pages/");
    // let root_page = String::from("https://bored.com/");
    // let root_page = String::from("https://www.math3ma.com/");
    // let root_page = String::from(
    //     "https://www.reddit.com/r/Blogging/comments/1josfud/your_favourite_blog_in_2025/",
    // );
    // let root_page =
    // String::from("https://www.theintrinsicperspective.com/p/writing-for-outlets-isnt-worth-it");
    // let root_page = String::from(
    //     "https://lithub.com/the-joy-and-privilege-of-growing-up-in-an-indie-bookstore/",
    // );
    //
    // let root_page = String::from("https://curlie.org");

    let mut app = Application::new(
        Box::new(ShuffleStrat::new(40, target_node_count)), //ToDo: think more about how to pick max at once
        Box::new(HtmlSelector::new()),
        Box::new(HtmlChecker::new()),
        Box::new(HashData::new()),
    );

    app.start(root_link)?;

    println!("Complete");
    Ok(())
}
