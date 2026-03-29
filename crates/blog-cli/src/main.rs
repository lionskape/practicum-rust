use std::path::PathBuf;

use anyhow::Result;
use blog_client::{BlogClient, Transport};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "blog-cli", about = "Blog platform CLI client")]
struct Cli {
    /// Use gRPC transport instead of HTTP
    #[arg(long)]
    grpc: bool,

    /// Server URL (HTTP: http://localhost:8080, gRPC: http://localhost:50051)
    #[arg(long, short)]
    server: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Register a new user
    Register {
        #[arg(long)]
        username: String,
        #[arg(long)]
        email: String,
        #[arg(long)]
        password: String,
    },
    /// Login with existing credentials
    Login {
        #[arg(long)]
        username: String,
        #[arg(long)]
        password: String,
    },
    /// Create a new blog post
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        content: String,
    },
    /// Get a blog post by ID
    Get {
        #[arg(long)]
        id: i64,
    },
    /// Update an existing blog post
    Update {
        #[arg(long)]
        id: i64,
        #[arg(long)]
        title: String,
        #[arg(long)]
        content: String,
    },
    /// Delete a blog post
    Delete {
        #[arg(long)]
        id: i64,
    },
    /// List blog posts
    List {
        #[arg(long)]
        limit: Option<i64>,
        #[arg(long)]
        offset: Option<i64>,
    },
}

fn token_path() -> PathBuf {
    dirs_or_default().join(".blog_token")
}

fn dirs_or_default() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn save_token(token: &str) -> Result<()> {
    std::fs::write(token_path(), token)?;
    Ok(())
}

fn load_token() -> Option<String> {
    std::fs::read_to_string(token_path()).ok().map(|s| s.trim().to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let transport = if cli.grpc {
        let addr = cli.server.unwrap_or_else(|| "http://localhost:50051".to_string());
        Transport::Grpc(addr)
    } else {
        let url = cli.server.unwrap_or_else(|| "http://localhost:8080".to_string());
        Transport::Http(url)
    };

    let mut client = BlogClient::new(transport).await?;

    if let Some(token) = load_token() {
        client.set_token(token);
    }

    match cli.command {
        Commands::Register { username, email, password } => {
            let resp = client.register(&username, &email, &password).await?;
            save_token(&resp.token)?;
            println!("Registered successfully!");
            println!(
                "User: {} (id: {}, email: {})",
                resp.user.username, resp.user.id, resp.user.email
            );
        }
        Commands::Login { username, password } => {
            let resp = client.login(&username, &password).await?;
            save_token(&resp.token)?;
            println!("Logged in successfully!");
            println!(
                "User: {} (id: {}, email: {})",
                resp.user.username, resp.user.id, resp.user.email
            );
        }
        Commands::Create { title, content } => {
            let post = client.create_post(&title, &content).await?;
            println!("Post created!");
            print_post(&post);
        }
        Commands::Get { id } => {
            let post = client.get_post(id).await?;
            print_post(&post);
        }
        Commands::Update { id, title, content } => {
            let post = client.update_post(id, &title, &content).await?;
            println!("Post updated!");
            print_post(&post);
        }
        Commands::Delete { id } => {
            client.delete_post(id).await?;
            println!("Post {id} deleted.");
        }
        Commands::List { limit, offset } => {
            let resp = client.list_posts(limit, offset).await?;
            println!("Posts ({} total):", resp.total);
            for post in &resp.posts {
                println!("---");
                print_post(post);
            }
        }
    }

    Ok(())
}

fn print_post(post: &blog_client::types::Post) {
    println!("[{}] {} (by {})", post.id, post.title, post.author_username);
    println!("{}", post.content);
    println!("Created: {} | Updated: {}", post.created_at, post.updated_at);
}
