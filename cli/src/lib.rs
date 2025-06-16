pub mod args;
pub mod col;
pub mod col_expr;
pub mod cols;
pub mod csv;
pub mod filter;
pub mod help;
pub mod json;
pub mod list_cols;
pub mod normal;
pub mod order;
pub mod sorting;
pub mod table;
pub mod units;
pub mod lustre;

use crate::lustre::{LustreData, MountLustreExt};

use {
    crate::{
        args::*,
        normal::*,
    },
    clap::Parser,
    std::{
        fs,
        os::unix::fs::MetadataExt,
    },
};


#[allow(clippy::match_like_matches_macro)]
pub fn run() {
    let args = Args::parse();
    if args.version {
        println!("dysk {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if args.help {
        help::print(args.ascii);
        csi_reset();
        return;
    }
    if args.list_cols {
        list_cols::print(args.color(), args.ascii);
        csi_reset();
        return;
    }
    let mut options = lfs_core::ReadOptions::default();
    options.remote_stats(args.remote_stats.unwrap_or_else(||true));
    let mut mounts = match lfs_core::read_mounts(&options) {
        Ok(mounts) => mounts,
        Err(e) => {
            eprintln!("Error reading mounts: {}", e);
            return;
        }
    };
    if !args.all {
        mounts.retain(is_normal);
    }
    
    let mut lustre_data = LustreData::new();
    if args.lustre || args.lustre_only || args.lustre_components {
        if let Err(e) = lustre_data.collect_lustre_info() {
            eprintln!("Warning: Failed to collect Lustre information: {}", e);
        }
    }
    
    // Add Lustre filtering if lustre_only is specified:
    if args.lustre_only {
        mounts.retain(|m| m.is_lustre(&lustre_data));
    }
    
    if let Some(path) = &args.path {
        let md = match fs::metadata(path) {
            Ok(md) => md,
            Err(e) => {
                eprintln!("Can't read {:?} : {}", path, e);
                return;
            }
        };
        let dev = lfs_core::DeviceId::from(md.dev());
        mounts.retain(|m| m.info.dev == dev);
    }
    if lustre_data.is_available {
        args.sort.sort_with_lustre(&mut mounts, &lustre_data);
    } else {
        args.sort.sort(&mut mounts);
    }
    let mounts = match args.filter.clone().unwrap_or_default().filter(&mounts) {
        Ok(mounts) => mounts,
        Err(e) => {
            eprintln!("Error in filter evaluation: {}", e);
            return;
        }
    };
    if args.csv {
        csv::print(&mounts, &args, &lustre_data).expect("writing csv failed");
        return;
    }
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json::output_value(&mounts, args.units, &lustre_data)).unwrap()
        );
        return;
    }
    if mounts.is_empty() {
        println!("no mount to display - try\n    dysk -a");
        return;
    }
    table::print(&mounts, args.color(), &args, &lustre_data);
    csi_reset();
}

/// output a Reset CSI sequence
fn csi_reset(){
    print!("\u{1b}[0m");
}
