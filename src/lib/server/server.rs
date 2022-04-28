use crate::lib::server::ServerOption;

use tokio::io::AsyncReadExt;
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

        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.option.port)).await?;

        loop {
            match listener.accept().await {
                Ok((mut socket, address)) => {
                    println!("# Connected: {}:{}", address.ip(), address.port());

                    tokio::spawn(async move {
                        let mut buffer = [0; 1024];

                        loop {
                            match socket.read(&mut buffer).await {
                                // socket closed
                                Ok(n) => {
                                    if n != 0 {
                                        let foo = String::from_utf8_lossy(&buffer);
                                        // TODO: 쿼리 실행 후 리턴값 반환
                                        println!("{}", foo);
                                    } else {
                                        eprintln!("# 연결 종료");
                                        break;
                                    }
                                }
                                Err(e) => {
                                    eprintln!("failed to read from socket; err = {:?}", e);
                                    break;
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
