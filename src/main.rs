use futures::future::join_all;
use reqwest;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::{io::Write, process::Command};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    pdf_url: String,
    language_to_translate_into: String,
    additional_prompts: Option<String>,
}

#[tokio::main]
async fn main() {
    let api_key = env::var("OPENAI_KEY").expect("OPENAI_KEY not set");
    let args = Args::parse();

    println!("Downloading pdf");
    download_pdf(args.pdf_url.as_str()).await.unwrap();
    println!("Pdf downloaded");

    let mut counter = 1;
    let mut futures = Vec::new();

    println!("Starting translations page by page");

    loop {
        let text_result = convert_pdf_page_to_text(counter, "file.pdf");

        match text_result {
            Ok(text) => {
                let future = translate_text_openai(
                    text.clone(),
                    api_key.clone(),
                    args.language_to_translate_into.clone(),
                );
                futures.push(future);
            }
            Err(_) => {
                // Handle the error or break out of the loop
                break;
            }
        }

        counter += 1;
    }

    let _ = std::fs::remove_file("file.pdf");

    println!("{} pages qeued for translation", futures.len());

    // Await all futures to complete concurrently
    let results = join_all(futures).await;

    println!("Translations collected... printing");

    // Combine the results
    let full_translated_text: String = results.iter().fold(String::new(), |mut acc, result| {
        if let Ok(text) = result {
            acc.push_str(text);
        }
        acc
    });

    println!("{}", full_translated_text);
}

async fn download_pdf(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await.unwrap();

    let pdf_bytes = response.bytes().await?;

    let mut file = std::fs::File::create("file.pdf")?;

    file.write_all(&pdf_bytes)?;

    Ok(())
}

#[derive(Debug)]
struct PdfParsingError(String);

impl std::fmt::Display for PdfParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for PdfParsingError {}

fn convert_pdf_page_to_text(page: i8, local_pdf_file: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("./pdftotext")
        .arg("-layout")
        .arg("-f")
        .arg(page.to_string())
        .arg("-l")
        .arg(page.to_string())
        .arg(local_pdf_file)
        // Don't specify an output -> write to stdout
        .arg("-")
        .output()?;

    if !output.status.success() {
        let error_message = format!(
            "pdftotext command failed with error: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        return Err(Box::new(PdfParsingError(error_message)));
    } else {
        let extracted_text = String::from_utf8_lossy(&output.stdout).into_owned();

        if extracted_text.is_empty() {
            return Err(Box::new(PdfParsingError("No text extracted".to_string())));
        }

        return Ok(extracted_text);
    }
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: i32,
    top_p: f32,
    frequency_penalty: i32,
    presence_penalty: i32,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

pub async fn translate_text_openai(
    text: String,
    api_key: String,
    language: String,
) -> Result<String, Box<dyn Error>> {
    let client = reqwest::Client::new();

    let system_message = Message {
        role: "system".to_string(),
        content: format!("Given the text below for a maths competition translate it in {}. Try to retain formulas when you can, usage of LateX is ok. Skip translating Rounds numbers, they are most likely at the bottom.", language).to_string(),
    };

    let user_message = Message {
        role: "user".to_string(),
        content: text.to_string(),
    };

    let request_payload = OpenAIRequest {
        model: "gpt-3.5-turbo".to_string(),
        messages: vec![system_message, user_message],
        temperature: 1.0,
        max_tokens: 2040,
        top_p: 1.0,
        frequency_penalty: 0,
        presence_penalty: 0,
    };

    let response: OpenAIResponse = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_payload)
        .send()
        .await?
        .json()
        .await?;

    let translated_text = response
        .choices
        .get(0)
        .map_or("".to_string(), |choice| choice.message.content.clone());

    Ok(translated_text)
}
