use crate::lib::server::ServerOption;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub struct Server {
    pub option: ServerOption,
}

impl Server {
    pub fn new(option: ServerOption) -> Self {
        Self { option }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("# Server is Running at {}", self.option.port);

        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.option.port)).await?;

        loop {
            match listener.accept().await {
                Ok((mut socket, address)) => {
                    println!("# Connected: {}:{}", address.ip(), address.port());

                    tokio::spawn(async move {
                        let mut buffer = [0; 1024];

                        // In a loop, read data from the socket and write the data back.
                        loop {
                            match socket.read(&mut buffer).await {
                                // socket closed
                                Ok(n) => {
                                    if n != 0 {
                                        let foo = String::from_utf8_lossy(&buffer);
                                        println!("{}", foo);
                                    }

                                    // Write the data back
                                    // if let Err(e) = socket.write_all(&buffer[0..n]).await {
                                    //     eprintln!("failed to write to socket; err = {:?}", e);
                                    //     return;
                                    // }
                                }
                                Err(e) => {
                                    eprintln!("failed to read from socket; err = {:?}", e);
                                }
                            };
                        }
                    });
                }
                Err(error) => {
                    println!("# Error: {}", error);
                }
            }
        }
    }
}
