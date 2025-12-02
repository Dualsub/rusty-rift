use clap::{Parser, Subcommand};
mod mesh;
mod texture;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Mesh {
        path: String,
        #[arg(short, long)]
        output: String,
    },
    Texture {
        path: String,
        #[arg(short, long)]
        output: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Mesh { path, output } => {
            mesh::load(&path, &output).expect("Failed to load mesh.")
        }
        Commands::Texture { path, output } => {
            texture::load(&path, &output).expect("Failed to load texture.")
        }
    }
}
