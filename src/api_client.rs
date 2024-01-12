use anyhow::anyhow;
use console::Emoji;
use crossbeam_channel::{unbounded, Receiver};
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use reqwest::Client;
use serde_json::{json, Value};
use solana_sdk::pubkey::Pubkey;
use std::thread;
use std::time::{Duration, Instant};

use crate::api_models::{JobResponse, JobStatus, JobVerificationResponse, VerifyResponse};

// Emoji constants
static DONE: Emoji<'_, '_> = Emoji("✅", "");
static WAITING: Emoji<'_, '_> = Emoji("⏳", "");
static ERROR: Emoji<'_, '_> = Emoji("❌", "X");

// URL for the remote server
pub const REMOTE_SERVER_URL: &str = "https://verify.osec.io";

fn loading_animation(receiver: Receiver<bool>) {
    let started = Instant::now();
    let spinner_style =
        ProgressStyle::with_template("[{elapsed_precise}] {prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");

    let pb = ProgressBar::new_spinner();
    pb.set_style(spinner_style);
    pb.set_message(format!(
        "Request sent. Awaiting server response. This may take a moment... {}",
        WAITING
    ));
    loop {
        match receiver.try_recv() {
            Ok(result) => {
                if result {
                    pb.finish_with_message(format!(
                        "{} Process completed. (Done in {})\n",
                        DONE,
                        HumanDuration(started.elapsed())
                    ));
                } else {
                    pb.finish_with_message(format!("{} Request processing failed.", ERROR));
                    println!(
                        "{} Time elapsed : {}",
                        ERROR,
                        HumanDuration(started.elapsed())
                    );
                }
                break;
            }

            Err(_) => {
                pb.inc(1);
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

// Send a job to the remote server
#[allow(clippy::too_many_arguments)]
pub async fn send_job_to_remote(
    repo_url: &str,
    commit_hash: &Option<String>,
    program_id: &Pubkey,
    library_name: &Option<String>,
    bpf_flag: bool,
    relative_mount_path: String,
    base_image: Option<String>,
    cargo_args: Vec<String>,
) -> anyhow::Result<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(18000))
        .build()?;

    // Send the POST request
    let response = client
        .post(format!("{}/verify", REMOTE_SERVER_URL))
        .json(&json!({
            "repository": repo_url,
            "commit_hash": commit_hash,
            "program_id": program_id.to_string(),
            "lib_name": library_name,
            "bpf_flag": bpf_flag,
            "mount_path":  if relative_mount_path.is_empty() {
                None
            } else {
                Some(relative_mount_path)
            },
            "base_image": base_image,
            "cargo_args": cargo_args,
        }))
        .send()
        .await?;

    if response.status().is_success() {
        let status_response: VerifyResponse = response.json().await?;
        println!("Verification request sent. {}", DONE);
        println!("Verification in progress... {}", WAITING);
        // Span new thread for polling the server for status
        // Create a channel for communication between threads
        let (sender, receiver) = unbounded();

        let handle = thread::spawn(move || loading_animation(receiver));

        // Poll the server for status
        loop {
            let status = check_job_status(&client, &status_response.request_id).await?;
            match status.status {
                JobStatus::InProgress => {
                    thread::sleep(Duration::from_secs(10));
                }
                JobStatus::Completed => {
                    let _ = sender.send(true);
                    handle.join().unwrap();
                    let status_response = status.respose.unwrap();
                    println!(
                        "Program {} has been successfully verified. {}",
                        program_id, DONE
                    );
                    println!("\nThe provided GitHub build matches the on-chain hash:");
                    println!("On Chain Hash: {}", status_response.on_chain_hash.as_str());
                    println!(
                        "Executable Hash: {}",
                        status_response.executable_hash.as_str()
                    );
                    println!("Repo URL: {}", status_response.repo_url.as_str());
                    break;
                }
                JobStatus::Failed => {
                    let _ = sender.send(false);

                    handle.join().unwrap();
                    let status_response: JobVerificationResponse = status.respose.unwrap();
                    println!("Program {} has not been verified. {}", program_id, ERROR);
                    eprintln!("Error message: {}", status_response.message.as_str());
                    break;
                }
                JobStatus::Unknown => {
                    let _ = sender.send(false);
                    handle.join().unwrap();
                    println!("Program {} has not been verified. {}", program_id, ERROR);
                    break;
                }
            }
        }

        Ok(())
    } else if response.status() == 409 {
        let status_response: Value = serde_json::from_str(&response.text().await?)?;

        if let Some(is_verified) = status_response["is_verified"].as_bool() {
            if is_verified {
                println!("Program {} has already been verified. {}", program_id, DONE);
                println!(
                    "On Chain Hash: {}",
                    status_response["on_chain_hash"].as_str().unwrap_or("")
                );
                println!(
                    "Executable Hash: {}",
                    status_response["executable_hash"].as_str().unwrap_or("")
                );
            } else {
                println!("This request has already been processed.");
                println!("Program {} has not been verified. {}", program_id, ERROR);
            }
        } else if status_response["status"] == "error" {
            println!(
                "Error message: {}",
                status_response["error"].as_str().unwrap_or("")
            );
        } else {
            println!("This request has already been processed.");
        }

        Ok(())
    } else {
        eprintln!("Encountered an error while attempting to send the job to remote");
        Err(anyhow!("{:?}", response.text().await?))?
    }
}

async fn check_job_status(client: &Client, request_id: &str) -> anyhow::Result<JobResponse> {
    // Get /job/:id
    let response = client
        .get(&format!("{}/job/{}", REMOTE_SERVER_URL, request_id))
        .send()
        .await
        .unwrap();

    if response.status().is_success() {
        // Parse the response
        let response: JobVerificationResponse = response.json().await?;
        match response.status {
            JobStatus::InProgress => {
                thread::sleep(Duration::from_secs(5));
                Ok(JobResponse {
                    status: JobStatus::InProgress,
                    respose: None,
                })
            }
            JobStatus::Completed => Ok(JobResponse {
                status: JobStatus::Completed,
                respose: Some(response),
            }),
            JobStatus::Failed => Ok(JobResponse {
                status: JobStatus::Failed,
                respose: Some(response),
            }),
            JobStatus::Unknown => Ok(JobResponse {
                status: JobStatus::Unknown,
                respose: Some(response),
            }),
        }
    } else {
        Err(anyhow!(
            "Encountered an error while attempting to check job status : {:?}",
            response.text().await?
        ))?
    }
}
