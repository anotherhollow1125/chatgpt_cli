use anyhow::Result;
use bytes::Bytes;
use dialoguer::Input;
use reqwest::{Client, RequestBuilder};
use tokio_stream::{Stream, StreamExt};

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
struct Choice {
    index: u64,
    message: Message,
    finish_reason: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ResponseBody {
    id: String,
    object: String,
    created: u64,
    choices: Vec<Choice>,
    usage: Usage,
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
            model: "gpt-4o".to_string(),
            messages: Vec::from(input_messages),
            stream: true,
        })
        .send()
        .await?
        .bytes_stream();

    Ok(res)
}

#[tokio::main]
async fn main() -> Result<()> {
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

    let input = Input::new()
        .with_prompt("You")
        .interact_text()
        .unwrap_or_else(|_| "quit".to_string());

    messages.push(Message {
        role: Role::User,
        content: input,
    });

    let mut response = query(&api_key, &messages).await?;

    while let Some(s) = response.next().await.transpose()? {
        println!("{:?}", String::from_utf8_lossy(&s));
    }

    Ok(())
}
