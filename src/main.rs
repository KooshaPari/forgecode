use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "forge")]
struct Args {
    #[arg(short, long)]
    watch: bool,
    #[arg(default_value = "test")]
    task: String,
}

fn main() {
    let args = Args::parse();
    println!("Running task: {}", args.task);
    if args.watch {
        println!("Watching for changes...");
    }
}
