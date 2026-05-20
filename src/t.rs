

pub struct Application {
    pub strategy: Box<dyn Strategy>,
    pub selector: Arc<dyn Selector>,
    pub auth: Arc<dyn Authenticator>,
    pub data: Box<dyn Data>,
    bureau: Bureaucracy,
}

impl Application {
    pub fn work(&mut self) -> Result<(), AnyErr> {
        while !self.strategy.end(&self.data) {
            //strategy is mutably borrowed here
            let next_nodes = self.strategy.next_nodes(&self.data);

            //this section was carefully designed to not need to mutate data.
            //except i have to keep some sort of ledger somehow if i want to rate limit.
            //Right now I am trying to do that with an interior mutabilitity pattern inside
            //Bureaucracy. perhaps there is another way to do it, like by sorting these links
            //or something.
            let mut async_tasks = Vec::new();
            for (action, link) in next_nodes {
                //this clone is necessary so it can be moved into its own async thread
                let bureau = self.bureau.clone();

                let task = spawn_async_task(async move {
                    match action {
                        Explore => {
                            /*snip */
                            bureau.get_url(&link).await;
                            /*snip */
                            Ok(result)
                        }
                        Validate => {
                            /*snip */
                            bureau.get_url(&link).await;
                            /*snip */
                            Ok(result)
                        }
                    }
                };
                async_tasks.push(task);
            }
            let action_results = wait_till_all_are_done(async_tasks);

            //mutably update the data with the results of all the network calls and parsing etc
            for ar in action_results {
                self.data.update(ar);
            }

        }

        write_dot_file(&self.data);
    }
}


//this is the struct that i am having to transition into having an interior mutability
//pattern


pub struct Bureaucracy {
    govs: HashMap<String, Govenor>,
    //This is really just a handle to network library black magic that we can clone on
    //a whim. We keep a copy here to seed the new Govenors
    client: request::Client,
}

pub struct Govenor {
    //immutable
    domain: String,
    client: reqwest::Client,
    robots_txt: Robot,
    rate: Duration,

    //mutable
    last_request: std::time::SystemTime,
}
