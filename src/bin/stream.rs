use anyhow::Result;
use bytes::Bytes;
use dialoguer::Input;
use reqwest::{Client, RequestBuilder};
use std::io;
use std::io::Write;
use tokio::io::AsyncBufReadExt;
use tokio_stream::{Stream, StreamExt};
use tokio_util::io::StreamReader;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct Message {
    role: Role,
    content: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RequestBody {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Content {
    content: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Choice {
    // index: u64,
    // message: Message,
    // finish_reason: String,
    delta: Content,
}

/*
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}
*/

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ResponseBody {
    // id: String,
    // object: String,
    // created: u64,
    choices: Vec<Choice>,
    // usage: Usage,
}

fn common_header(api_key: &str) -> RequestBuilder {
    let api_key_field = format!("Bearer {}", api_key);

    Client::new()
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", api_key_field.as_str())
}

async fn query(
    api_key: &str,
    input_messages: &[Message],
) -> Result<impl Stream<Item = reqwest::Result<Bytes>>> {
    let res = common_header(api_key)
        .json(&RequestBody {
            model: "gpt-4".to_string(),
            messages: Vec::from(input_messages),
            stream: true,
        })
        .send()
        .await?
        .bytes_stream();

    Ok(res)
}

fn to_response(line: String) -> Result<ResponseBody> {
    let line = line.replace("data: ", "");

    let response_body: ResponseBody = serde_json::from_str(&line)?;

    Ok(response_body)
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut stdout = std::io::stdout();
    dotenvy::dotenv().ok();

    let api_key = std::env::var("CHATGPT_APIKEY")?;

    if api_key.is_empty() {
        eprintln!("Please set the environment variable CHATGPT_APIKEY");
        std::process::exit(1);
    }

    let mut messages = vec![Message {
        role: Role::System,
        content: "You are a helpful assistant.".to_string(),
    }];

    loop {
        let input = Input::new()
            .with_prompt("You")
            .interact_text()
            .unwrap_or_else(|_| "quit".to_string());

        if input == "quit" || input == "q" {
            break;
        }

        messages.push(Message {
            role: Role::User,
            content: input,
        });

        let response = query(&api_key, &messages).await?;
        let mut response = StreamReader::new(
            response.map(|r| r.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))),
        );

        print!("ChatGPT: ");
        stdout.flush()?;

        let mut line = Vec::new();
        let mut all_content = "ChatGPT: ".to_string();
        while response.read_until(b'\n', &mut line).await? > 0 {
            let line_str = String::from_utf8_lossy(&line);
            let response_body_str = line_str.trim().to_string();
            line.clear();

            if let Ok(response_body) = to_response(response_body_str.to_string()) {
                if let Some(c) = response_body.choices.first() {
                    let content_parts = &c.delta.content;

                    print!("{}", content_parts);
                    stdout.flush()?;

                    all_content.push_str(content_parts);
                }
            }
        }

        println!();

        messages.push(Message {
            role: Role::Assistant,
            content: all_content,
        });
    }

    Ok(())
}
