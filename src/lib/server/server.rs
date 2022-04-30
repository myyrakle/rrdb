use crate::lib::ast::{DDLStatement, SQLStatement};
use crate::lib::server::ServerOption;
use crate::lib::Executor;
use crate::lib::Parser;

use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

pub struct Server {
    pub option: ServerOption,
}

async fn process_query(query: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = Parser::new(query);
    let executor = Executor::new();

    let ast_list = parser.parse().unwrap();

    for ast in ast_list {
        match ast {
            SQLStatement::DDL(DDLStatement::CreateDatabaseQuery(query)) => {
                return Ok(executor.create_database(query).await?);
            }
            _ => {
                println!("?: {:?}", ast);
            }
        }
    }

    Ok(())
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
                                        let query = String::from_utf8_lossy(&buffer).to_string();
                                        println!("QUERY: {}", query);

                                        match process_query(query.clone()).await {
                                            Ok(_) => {
                                                let response = "OK".to_string();
                                                println!("RESPONSE: {}", response);
                                            }
                                            Err(error) => {
                                                println!("ERROR: {}", error);
                                            }
                                        }
                                        // TODO: 쿼리 실행 후 리턴값 반환
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
