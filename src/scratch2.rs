// use futures::{
//     future::{BoxFuture, FutureExt},
//     task::{waker_ref, ArcWake},
// };
use std::{
    collections::VecDeque,
    future::Future,
    sync::{
        mpsc::{channel, Receiver, SyncSender},
        Arc, Mutex,
    },
    task::Context,
    time::Duration,
};

pub struct Submission {
    sender: oneshot::Sender<String>,
    page: String,
}

//is a future
pub struct Reply {
    reciver: oneshot::Receiver<String>,
}

impl Future for Reply {
    type Output = String;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

pub fn new_pair(page: String) -> (Submission, Reply) {
    let (tx, rx) = oneshot::channel();
    (
        Submission {
            sender: tx,
            page: page,
        },
        Reply { reciver: rx },
    )
}

pub struct GovHandle {
    // core: Arc<Mutex<GovCore>>
    pub sender: std::sync::mpsc::Sender<Submission>,
}

// impl GovHandle {
//     // pub fn submit_request(&mut self, page: String) -> impl Future<Output = Reply> {
//     //     let mut core = self.core.lock().unwrap();
//     //     let (submission, reply) = new_pair(page);
//     //     core.push_back(submission);
//     //     reply
//     // }

// }

pub struct GovCore {
    recv: std::sync::mpsc::Receiver<Submission>,
    client: reqwest::blocking::Client,
    domain: String,
    /*
     * all the tracking and timing fields
     */
}

impl GovCore {
    pub fn create_govenor(fields: T) -> GovHandle {
        let (sender, recv) = channel();
        let client = reqwest::blocking::Client::new();
        let core = GovCore::init_core(recv, client);
        core.start();
        GovHandle { sender }
    }

    fn init_core(recv: Receiver<Submission>, client: reqwest::blocking::Client) -> GovCore {
        GovCore { recv, client }
    }

    pub fn start(self) {
        std::thread::Builder::new()
            .name(format!("{}-govenor", self.domain))
            .spawn(move || loop {
                match self.recv.recv() {
                    Err(e) => {
                        println!("Error: {e}");
                        break;
                    }
                    Ok(submission) => {
                        let ret = {
                            let mut tries = 0;
                            while tries < max_tries {
                                wait_until_allowed();
                                resp = make_request(submission.page); // blocking;
                                if resp.is_not_err() {
                                    Ok(resp.body())
                                } else {
                                    continue;
                                }
                            }
                            Err(timeout)
                        };

                        submission.sender.send(ret);
                    }
                }
            });
    }
}
