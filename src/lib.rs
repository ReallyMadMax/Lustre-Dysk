// lib.rs - Library interface for dysk
//
// This file exposes dysk's functionality as a library so it can be used
// as a dependency in other projects.

// Re-export all the core modules
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

// Re-export commonly used types for easier access
pub use args::{Args, TriBool};
pub use col::Col;
pub use cols::Cols;
pub use filter::Filter;
pub use normal::is_normal;
pub use order::Order;
pub use sorting::Sorting;
pub use units::Units;

// Re-export the main functionality
use {
    std::{
        fs,
        os::unix::fs::MetadataExt,
    },
};

use std::fmt::Debug;
use crate::table::{print_generic_csv, print_generic_json};

// Generic trait that any mount-like structure can implement
pub trait MountLike: Debug + Clone {
    fn filesystem_name(&self) -> String;
    fn mount_point(&self) -> String;
    fn total_bytes(&self) -> Option<u64>;
    fn used_bytes(&self) -> Option<u64>;
    fn available_bytes(&self) -> Option<u64>;
    fn usage_percentage(&self) -> Option<f64>;
    fn filesystem_type(&self) -> String;
    fn is_normal(&self) -> bool;
}

// Implement for the existing lfs_core::Mount
impl MountLike for lfs_core::Mount {
    fn filesystem_name(&self) -> String {
        if self.info.fs.is_empty() {
            format!("{}:{}", self.info.dev.major, self.info.dev.minor)
        } else {
            self.info.fs.clone()
        }
    }
    
    fn mount_point(&self) -> String {
        self.info.mount_point.to_string_lossy().to_string()
    }
    
    fn total_bytes(&self) -> Option<u64> {
        self.stats().map(|s| s.size())
    }
    
    fn used_bytes(&self) -> Option<u64> {
        self.stats().map(|s| s.used())
    }
    
    fn available_bytes(&self) -> Option<u64> {
        self.stats().map(|s| s.available())
    }
    
    fn usage_percentage(&self) -> Option<f64> {
        self.stats().map(|s| s.use_share() * 100.0)
    }
    
    fn filesystem_type(&self) -> String {
        self.info.fs_type.clone()
    }
    
    fn is_normal(&self) -> bool {
        normal::is_normal(self)
    }
}

// Generic table printing function
pub fn print_generic_table<T: MountLike>(
    mounts: &[T], 
    args: &Args
) -> Result<(), Box<dyn std::error::Error>> {
    if args.csv {
        print_generic_csv(mounts, args)?;
    } else if args.json {
        print_generic_json(mounts, args)?;
    } else if mounts.is_empty() {
        println!("no mounts to display");
    }
    Ok(())
}


/// Core dysk functionality as a library function
/// This allows other crates to use dysk's logic programmatically
pub fn get_mounts(args: &Args) -> Result<Vec<lfs_core::Mount>, Box<dyn std::error::Error>> {
    let mut options = lfs_core::ReadOptions::default();
    options.remote_stats(args.remote_stats.unwrap_or_else(|| true));
    
    let mut mounts = lfs_core::read_mounts(&options)?;
    
    if !args.all {
        mounts.retain(is_normal);
    }
    
    if let Some(path) = &args.path {
        let md = fs::metadata(path)?;
        let dev = lfs_core::DeviceId::from(md.dev());
        mounts.retain(|m| m.info.dev == dev);
    }
    
    args.sort.sort(&mut mounts);
    
    Ok(mounts)
}

/// Get filtered mounts based on args
pub fn get_filtered_mounts(args: &Args) -> Result<Vec<lfs_core::Mount>, Box<dyn std::error::Error>> {
    let mounts = get_mounts(args)?;
    
    if let Some(filter) = &args.filter {
        let filtered = filter.filter(&mounts)?;
        Ok(filtered.into_iter().cloned().collect())
    } else {
        Ok(mounts)
    }
}

/// Print output in the specified format
pub fn print_output(mounts: &[lfs_core::Mount], args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let mount_refs: Vec<&lfs_core::Mount> = mounts.iter().collect();
    
    if args.csv {
        csv::print(&mount_refs, args)?;
    } else if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json::output_value(&mount_refs, args.units))?
        );
    } else if mount_refs.is_empty() {
        println!("no mount to display - try\n    dysk -a");
    } else {
        table::print(&mount_refs, args.color(), args);
    }
    
    Ok(())
}

/// Main dysk functionality as a library function
/// This replicates the main dysk logic but returns results instead of printing and exiting
pub fn run_dysk(args: Args) -> Result<Vec<lfs_core::Mount>, Box<dyn std::error::Error>> {
    if args.version {
        println!("dysk {}", env!("CARGO_PKG_VERSION"));
        return Ok(Vec::new());
    }
    
    if args.help {
        help::print(args.ascii);
        return Ok(Vec::new());
    }
    
    if args.list_cols {
        list_cols::print(args.color(), args.ascii);
        return Ok(Vec::new());
    }
    
    let mounts = get_filtered_mounts(&args)?;
    print_output(&mounts, &args)?;
    
    Ok(mounts)
}

/// Original main function for binary compatibility
pub fn run() {
    use clap::Parser;
    
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
    options.remote_stats(args.remote_stats.unwrap_or_else(|| true));
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
    
    args.sort.sort(&mut mounts);
    
    let mount_refs = match args.filter.clone().unwrap_or_default().filter(&mounts) {
        Ok(mounts) => mounts,
        Err(e) => {
            eprintln!("Error in filter evaluation: {}", e);
            return;
        }
    };
    
    if args.csv {
        csv::print(&mount_refs, &args).expect("writing csv failed");
        return;
    }
    
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json::output_value(&mount_refs, args.units)).unwrap()
        );
        return;
    }
    
    if mounts.is_empty() {
        println!("no mount to display - try\n    dysk -a");
        return;
    }
    
    table::print(&mount_refs, args.color(), &args);
    csi_reset();
}

/// output a Reset CSI sequence
fn csi_reset() {
    print!("\u{1b}[0m");
}

// Extension traits and helper functions for library users

/// Trait to extend Mount with additional dysk functionality
pub trait MountExt {
    /// Check if this mount should be shown in normal dysk output
    fn is_normal(&self) -> bool;
    
    /// Get a display name for this mount
    fn display_name(&self) -> String;
    
    /// Get usage percentage as a float (0.0 to 1.0)
    fn usage_percentage(&self) -> Option<f64>;
}

impl MountExt for lfs_core::Mount {
    fn is_normal(&self) -> bool {
        is_normal(self)
    }
    
    fn display_name(&self) -> String {
        if self.info.fs.is_empty() {
            format!("{}:{}", self.info.dev.major, self.info.dev.minor)
        } else {
            self.info.fs.clone()
        }
    }
    
    fn usage_percentage(&self) -> Option<f64> {
        self.stats().map(|s| s.use_share())
    }
}

/// Builder pattern for dysk arguments
#[derive(Debug, Default)]
pub struct ArgsBuilder {
    args: Args,
}

impl ArgsBuilder {
    pub fn new() -> Self {
        Self {
            args: Args {
                help: false,
                version: false,
                all: false,
                color: TriBool::Auto,
                ascii: false,
                remote_stats: TriBool::Auto,
                list_cols: false,
                cols: Cols::default(),
                filter: None,
                sort: Sorting::default(),
                units: Units::default(),
                json: false,
                csv: false,
                csv_separator: ',',
                path: None,
            },
        }
    }
    
    pub fn all(mut self, all: bool) -> Self {
        self.args.all = all;
        self
    }
    
    pub fn json(mut self, json: bool) -> Self {
        self.args.json = json;
        self
    }
    
    pub fn csv(mut self, csv: bool) -> Self {
        self.args.csv = csv;
        self
    }
    
    pub fn filter<F: Into<Filter>>(mut self, filter: F) -> Self {
        self.args.filter = Some(filter.into());
        self
    }
    
    pub fn sort<S: Into<Sorting>>(mut self, sort: S) -> Self {
        self.args.sort = sort.into();
        self
    }
    
    pub fn units(mut self, units: Units) -> Self {
        self.args.units = units;
        self
    }
    
    pub fn path<P: Into<std::path::PathBuf>>(mut self, path: P) -> Self {
        self.args.path = Some(path.into());
        self
    }
    
    pub fn cols<C: Into<Cols>>(mut self, cols: C) -> Self {
        self.args.cols = cols.into();
        self
    }
    
    pub fn build(self) -> Args {
        self.args
    }
}

// Convenience functions for common use cases

/// Get all mounts in JSON format
pub fn get_mounts_json() -> Result<String, Box<dyn std::error::Error>> {
    let args = ArgsBuilder::new().json(true).build();
    let mounts = get_mounts(&args)?;
    let mount_refs: Vec<&lfs_core::Mount> = mounts.iter().collect();
    Ok(serde_json::to_string_pretty(&json::output_value(&mount_refs, args.units))?)
}

/// Get all normal mounts (filtered like default dysk output)
pub fn get_normal_mounts() -> Result<Vec<lfs_core::Mount>, Box<dyn std::error::Error>> {
    let args = ArgsBuilder::new().build();
    get_mounts(&args)
}

/// Get mounts for a specific path
pub fn get_mounts_for_path<P: Into<std::path::PathBuf>>(path: P) -> Result<Vec<lfs_core::Mount>, Box<dyn std::error::Error>> {
    let args = ArgsBuilder::new().path(path).build();
    get_mounts(&args)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_args_builder() {
        let args = ArgsBuilder::new()
            .all(true)
            .json(true)
            .units(Units::Binary)
            .build();
        
        assert!(args.all);
        assert!(args.json);
        assert_eq!(args.units, Units::Binary);
    }
    
    #[test]
    fn test_get_mounts_basic() {
        let args = ArgsBuilder::new().build();
        // This test will fail in environments without proper mounts,
        // but it demonstrates the API
        let _result = get_mounts(&args);
    }
}