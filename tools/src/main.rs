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
        #[arg(short = 'x', long = "resize-width")]
        resize_width: Option<u32>,
        #[arg(short = 'y', long = "resize-height")]
        resize_height: Option<u32>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Mesh { path, output } => {
            mesh::load(&path, &output).expect("Failed to load mesh.")
        }
        Commands::Texture {
            path,
            output,
            resize_width,
            resize_height,
        } => texture::load(&texture::TextureLoadDesc {
            path: &path,
            output: &output,
            resize_width: *resize_width,
            resize_height: *resize_height,
        })
        .expect("Failed to load texture."),
    }
}
