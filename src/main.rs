use crate::all_packages::AllPackages;
mod args;
mod index;
mod unit;
use index::GlobalIndex;
use args::Args;
use clap::Parser;
use unit::check_unit_dir;

mod all_packages;
mod line_index;

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

    // println!("{:#?}", reference_index);

    // Function that parses all-packages.nix, returning a struct for every identifier assignment
    // that could be migrated, without looking at the file references

    let mut ap = AllPackages::new(&cli.path.join("pkgs/top-level/all-packages.nix"));

    for (key, _) in ap.entries.to_owned() {
        // To test, remove all packages from all-packages.nix
        ap.remove(&key);
    }

    ap.render();




}

