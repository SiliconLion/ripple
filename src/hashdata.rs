use crate::interface::CrawlState::*;
use crate::interface::*;
use crate::Link;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct HashData {
    //domain -> all the pages in that domain
    data: HashMap<String, Vec<WebNode>>,
}

impl Data for HashData {
    fn get(&self, link: &Link) -> &WebNode {
        let nodes = self.data.get(&link.domain).unwrap();
        for node in nodes {
            if node.link == *link {
                return &node;
            }
        }
        panic! {"Cannot find node with that link"}
    }

    // fn get_mut(&mut self, link: &Link) -> Result<&mut WebNode, AnyErr> {
    //     let nodes = self.data.get_mut(&link.domain)?;
    //     for node in nodes {
    //         if node.link == link {
    //             return Ok(&mut node);
    //         }
    //     }
    //     bail! {"Cannot find node with that link"}
    // }

    fn add(&mut self, node: WebNode) {
        if !self.data.contains_key(&node.link.domain) {
            self.data.insert(node.link.domain.clone(), Vec::new());
        }
        let domain = self.data.get_mut(&node.link.domain).unwrap();
        let mut contains = false;
        for e in domain.iter() {
            if e.link == node.link {
                contains = true;
                break;
            }
        }
        if !contains {
            domain.push(node);
        }
    }

    //potentially very expensive
    fn remove(&mut self, link: Link) {
        let domain = self.data.get_mut(&link.domain).unwrap();
        let mut idx = usize::MAX; //rust wants us to initialize variables so
        let mut found = false;
        for (i, node) in domain.iter_mut().enumerate() {
            if node.link == link && found == false {
                found = true;
                idx = i;
            }

            match &mut node.state {
                Explored(links) => {
                    if let Some(j) = links.iter().position(|l| *l == link) {
                        links.remove(j);
                    }
                }
                _ => {}
            }
        }

        if found {
            domain.remove(idx);
        } else {
            panic!("cannot remove node with link because it is not present")
        }
    }

    fn neighbors(&self, link: Link) -> Vec<Link> {
        let node = self.get(&link);
        match &node.state {
            Explored(links) => links.clone(),
            _ => Vec::new(),
        }
    }

    fn all_nodes(&self) -> Vec<Link> {
        self.data
            .values() // Iterator<Vec<WebNode>>
            .map(|v| v.iter()) // Iterator<&Iterator<&WebNode>>
            .flatten() // Iterator<&WebNode>
            .map(|node| node.link.clone()) //Iterator<Link>
            .collect() //Vec<Link>
    }

    fn total_nodes(&self) -> usize {
        self.all_nodes().len()
    }

    fn domain_names(&self) -> Vec<String> {
        self.data.keys().map(|k| k.clone()).collect()
    }

    fn get_domain(&self, domain: String) -> Vec<Link> {
        if let Some(v) = self.data.get(&domain) {
            let links = v.iter().map(|node| node.link.clone()).collect();
            return links;
        } else {
            return Vec::new();
        }
    }

    fn update(&mut self, res: ActionResult) {
        let node = self.get_node_mut(&res.link);
        node.state = res.state.clone();

        match &res.state {
            Explored(links) => {
                for l in links {
                    self.add(WebNode::new_canidate(l));
                }
            }
            _ => {}
        }
    }

    fn represent(&self) -> String {
        self.dot_pages()
        // format!("{:?}", self.data)

        // let mut s = String::from("WebMap: {\n");
        // for domain in self.data.keys() {
        //     s.push_str(&domain);
        //     s.push_str(" : [");
        //     for node in self.data.get(domain).unwrap() {
        //         s.push_str(&node.link.as_string());
        //         s.push_str(" ");
        //     }
        //     s.push_str("]\n");
        // }
        // s.push_str("}\n");
        // s
    }

    fn print_stats(&self) {
        let domain_count = self.data.keys().len();

        let all_nodes = self.all_nodes();
        let node_count = all_nodes.len();

        let mut canidate_count = 0;
        let mut stub_count = 0;
        let mut forbidden_count = 0;
        let mut rejected_count = 0;
        let mut verified_count = 0;
        let mut explored_count = 0;
        let mut failed_count = 0;

        //ToDo: without rstfm this could be so many fewer lines lol
        for node_name in all_nodes {
            let node = self.get(&node_name);
            match node.state {
                Canidate => {
                    canidate_count += 1;
                }
                Stub => {
                    stub_count += 1;
                }
                Forbidden => {
                    forbidden_count += 1;
                }
                Verified => {
                    rejected_count += 1;
                }
                Rejected => {
                    verified_count += 1;
                }
                Explored(_) => {
                    explored_count += 1;
                }
                Failed => {
                    failed_count += 1;
                }
            }
        }

        println!("Domains:   {domain_count}");
        println!("Nodes:     {node_count}");
        println!("Canidates: {canidate_count}");
        println!("Stubs:     {stub_count}");
        println!("Forbidden: {forbidden_count}");
        println!("Rejected:  {rejected_count}");
        println!("Verified:  {verified_count}");
        println!("Explored:  {explored_count}");
        println!("Failed:    {failed_count}");
        println!("\n\n");
    }
}

impl HashData {
    fn enumerate_link(map: &mut HashMap<Link, String>, link: &Link, counter: &mut usize) {
        if map.contains_key(&link) {
            return;
        }
        let id = format!("node_{}", counter);
        map.insert(link.clone(), id);
        *counter += 1;
    }

    pub fn dot_pages(&self) -> String {
        use crate::interface::CrawlState::*;

        // let mut link_ids: HashMap<Link, String> = HashMap::new();
        // let mut counter = 0;
        // for link in self.all_nodes() {
        //     HashData::enumerate_link(&mut link_ids, &link, &mut counter);
        //     match &self.get(&link).state {
        //         Explored(list) => {
        //             for e in list {
        //                 HashData::enumerate_link(&mut link_ids, &e, &mut counter);
        //             }
        //         }
        //         _ => {}
        //     }
        // }

        //enumerate all links contained in self.data - begin

        //first enumerate all links that are nodes in self.data
        let mut counter = 0;
        let mut link_ids = HashMap::new();

        let all_nodes = self.all_nodes();
        for node_link in &all_nodes {
            HashData::enumerate_link(&mut link_ids, &node_link, &mut counter);
        }

        //then for every node, if it has been explored, enumerate what it links to
        for node_name in &all_nodes {
            let node = self.get(node_name);
            let connections = match &node.state {
                Explored(c) => c,
                _ => &Vec::new(),
            };

            //(connections may obviously be empty)
            for c in connections {
                HashData::enumerate_link(&mut link_ids, c, &mut counter);
            }
        }

        //end enumeration

        //construct dot file

        let mut dot = String::from("digraph WebMapPages {\n");
        for node_name in self.all_nodes() {
            let line = match &self.get(&node_name).state {
                Explored(connections) => {
                    let mut ln = link_ids.get(&node_name).unwrap().clone();
                    ln.push_str(" -> { ");

                    for c in connections {
                        let c_id = link_ids.get(c).unwrap();
                        ln += c_id;
                        ln.push_str(&" ");
                    }
                    ln.push_str("}\n");
                    ln
                }
                _ => link_ids.get(&node_name).unwrap().clone(),
            };
            dot += &line;
            dot.push_str("\n");
        }

        dot.push_str("\n}\n");
        return dot;
    }

    // pub fn dot_domains(&self) -> String {}
    //
    pub fn new() -> HashData {
        HashData {
            data: HashMap::new(),
        }
    }

    fn get_node_mut(&mut self, link: &Link) -> &mut WebNode {
        let domain = self.data.get_mut(&link.domain).unwrap();
        let node: &mut WebNode = domain.iter_mut().find(|n| n.link == *link).unwrap();
        node
    }
}

// pub struct GraphData {
//     data: StableDiGraph<WebNode, ()>,
//     keys: HashMap<String>,
// }
