mod args;
mod index;
mod unit;
use index::GlobalIndex;
use args::Args;
use clap::Parser;
use unit::check_unit_dir;

fn main() {

    let cli = Args::parse();

    let reference_index = GlobalIndex::new(&cli.path);

    let unit_dir = cli.path.join("pkgs/unit");
    if ! unit_dir.exists() {
        eprintln!("Unit directory doesn't exist, skipping check");
    } else {
        check_unit_dir(unit_dir, &reference_index);
        eprintln!("Unit directory is valid");
    }

    println!("{:#?}", reference_index);

}

