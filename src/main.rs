// Indicate that we have another module
mod worker;

use tokio::{fs::File, io::AsyncBufReadExt, io::BufReader};

/// Create a new error type to handle the two ways errors can happen.
#[derive(Debug)]
enum AppError {
    IO(std::io::Error),
    Reqwest(reqwest::Error),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::IO(e) => write!(f, "IO Error: {}", e),
            AppError::Reqwest(e) => write!(f, "Reqwest Error: {}", e),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::IO(e)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Reqwest(e)
    }
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    const JUMLAH_WORKER: usize = 4;

    let file = File::open("urls.csv").await?;
    let buffer_file = BufReader::new(file);
    let mut lines = buffer_file.lines();

    println!("Alias, Status");

    let klien = reqwest::Client::new();
    let mut workers = worker::Robot::new();

    let (tx, rx) = async_channel::bounded(JUMLAH_WORKER * 2);

    // Spawn the task to fill up the queue
    workers.spawn(async move {
        while let Some(line) = lines.next_line().await? {
            if let Some((alias, url)) = line.split_once(',') {
                tx.send((alias.to_string(), url.to_string())).await.unwrap()
            }
        }
        Ok(())
    });

    // Spawn off the individual workers
    for _ in 0..JUMLAH_WORKER {
        let klien = klien.clone();
        let rx = rx.clone();
        workers.spawn(async move {
            loop {
                match rx.recv().await {
                    // uses Err to represent a closed channel due to tx being dropped
                    Err(_) => break Ok(()),
                    Ok((alias, url)) => {
                        let resp = klien
                            .get(&url)
                            .timeout(std::time::Duration::from_secs(5))
                            .send()
                            .await;
                        match resp {
                            Ok(r) => println!("{},{}", alias, r.status().as_u16()),
                            Err(e) => println!("{},{}", alias, e),
                        };
                    }
                }
            }
        })
    }

    // Wait for the workers to complete
    workers.run().await
}
