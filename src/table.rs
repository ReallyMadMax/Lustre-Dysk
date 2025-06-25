use {
    crate::{
        Args, col::Col,
    },
    lfs_core::*,
    termimad::{
        crossterm::style::Color::*,
        minimad::{self, OwningTemplateExpander, TableBuilder},
        CompoundStyle, MadSkin, ProgressBar,
    },
};
use crate::MountLike;

// those colors are chosen to be "redish" for used, "greenish" for available
// and, most importantly, to work on both white and black backgrounds. If you
// find a better combination, please show me.
static USED_COLOR: u8 = 209;
static AVAI_COLOR: u8 = 65;
static SIZE_COLOR: u8 = 172;

static BAR_WIDTH: usize = 5;
static INODES_BAR_WIDTH: usize = 5;

pub mod table {
    use crate::{MountLike, Units};
    use super::*;
    
    pub fn print_generic<T: MountLike>(mounts: &[T], use_color: bool, args: &Args) {
        print_header_generic(use_color);
        
        for mount in mounts {
            print_row_generic(mount, use_color, args);
        }
        
        if use_color {
            print!("\x1b[0m"); // Reset colors
        }
    }
    
    fn print_header_generic(use_color: bool) {
        if use_color {
            print!("\x1b[1m"); // Bold
        }
        
        println!("{:<30} {:>8} {:>8} {:>8} {:>5} {:>8} {}",
            "filesystem", "type", "size", "used", "use%", "avail", "mounted on");
        
        if use_color {
            print!("\x1b[0m"); // Reset
        }
    }
    
    fn print_row_generic<T: MountLike>(mount: &T, use_color: bool, args: &Args) {
        let fs_name = truncate_string(&mount.filesystem_name(), 30);
        let fs_type = mount.filesystem_type();
        
        if let (Some(total), Some(used), Some(avail)) = 
            (mount.total_bytes(), mount.used_bytes(), mount.available_bytes()) {
            
            let (size_str, used_str, avail_str) = format_sizes(total, used, avail, args.units);
            
            let usage_pct = mount.usage_percentage().unwrap_or(0.0);
            let usage_str = format!("{:.0}%", usage_pct);
            
            let colored_usage = if use_color {
                if usage_pct >= 90.0 {
                    format!("\x1b[31m{}\x1b[0m", usage_str) // Red
                } else if usage_pct >= 75.0 {
                    format!("\x1b[33m{}\x1b[0m", usage_str) // Yellow
                } else {
                    format!("\x1b[32m{}\x1b[0m", usage_str) // Green
                }
            } else {
                usage_str
            };
            
            println!("{:<30} {:>8} {:>8} {:>8} {:>5} {:>8} {}",
                fs_name,
                fs_type,
                size_str,
                used_str,
                colored_usage,
                avail_str,
                mount.mount_point());
        } else {
            // Handle inactive/unmounted filesystems
            let inactive = if use_color { "\x1b[90m-\x1b[0m" } else { "-" };
            
            println!("{:<30} {:>8} {:>8} {:>8} {:>5} {:>8} {}",
                fs_name,
                fs_type,
                inactive,
                inactive,
                inactive,
                inactive,
                mount.mount_point());
        }
    }
    
    fn truncate_string(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }
    
    fn format_sizes(total: u64, used: u64, avail: u64, units: Units) -> (String, String, String) {
        match units {
            Units::Binary => (
                format_bytes_binary(total),
                format_bytes_binary(used),
                format_bytes_binary(avail),
            ),
            Units::Si => (
                format_bytes_si(total),
                format_bytes_si(used),
                format_bytes_si(avail),
            ),
            Units::Bytes => (
                total.to_string(),
                used.to_string(),
                avail.to_string(),
            ),
        }
    }
    
    fn format_bytes_binary(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "K", "M", "G", "T", "P"];
        let mut size = bytes as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        if unit_index == 0 {
            format!("{:.0}{}", size, UNITS[unit_index])
        } else if size >= 100.0 {
            format!("{:.0}{}", size, UNITS[unit_index])
        } else {
            format!("{:.1}{}", size, UNITS[unit_index])
        }
    }
    
    fn format_bytes_si(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "K", "M", "G", "T", "P"];
        let mut size = bytes as f64;
        let mut unit_index = 0;
        
        while size >= 1000.0 && unit_index < UNITS.len() - 1 {
            size /= 1000.0;
            unit_index += 1;
        }
        
        if unit_index == 0 {
            format!("{:.0}{}", size, UNITS[unit_index])
        } else if size >= 100.0 {
            format!("{:.0}{}", size, UNITS[unit_index])
        } else {
            format!("{:.1}{}", size, UNITS[unit_index])
        }
    }
}

pub(crate) fn print_generic_csv<T: MountLike>(mounts: &[T], args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("filesystem,type,total_bytes,used_bytes,available_bytes,usage_percent,mount_point");
    
    for mount in mounts {
        println!("{},{},{},{},{},{:.2},{}",
            mount.filesystem_name(),
            mount.filesystem_type(),
            mount.total_bytes().unwrap_or(0),
            mount.used_bytes().unwrap_or(0),
            mount.available_bytes().unwrap_or(0),
            mount.usage_percentage().unwrap_or(0.0),
            mount.mount_point());
    }
    
    Ok(())
}

pub(crate) fn print_generic_json<T: MountLike>(mounts: &[T], args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    use serde_json::{json, Value};
    
    let mount_data: Vec<Value> = mounts.iter().map(|mount| {
        json!({
            "filesystem": mount.filesystem_name(),
            "type": mount.filesystem_type(),
            "total_bytes": mount.total_bytes(),
            "used_bytes": mount.used_bytes(),
            "available_bytes": mount.available_bytes(),
            "usage_percentage": mount.usage_percentage(),
            "mount_point": mount.mount_point()
        })
    }).collect();
    
    let output = json!({ "mounts": mount_data });
    println!("{}", serde_json::to_string_pretty(&output)?);
    
    Ok(())
}

pub fn print(mounts: &[&Mount], color: bool, args: &Args) {
    if args.cols.is_empty() {
        return;
    }
    let units = args.units;
    let mut expander = OwningTemplateExpander::new();
    expander.set_default("");
    for mount in mounts {
        let sub = expander
            .sub("rows")
            .set("id", mount.info.id)
            .set("dev-major", mount.info.dev.major)
            .set("dev-minor", mount.info.dev.minor)
            .set("filesystem", &mount.info.fs)
            .set("disk", mount.disk.as_ref().map_or("", |d| d.disk_type()))
            .set("type", &mount.info.fs_type)
            .set("mount-point", mount.info.mount_point.to_string_lossy())
            .set_option("uuid", mount.uuid.as_ref())
            .set_option("part_uuid", mount.part_uuid.as_ref());
        if let Some(label) = &mount.fs_label {
            sub.set("label", label);
        }
        if mount.info.is_remote() {
            sub.set("remote", "x");
        }
        if let Some(stats) = mount.stats() {
            let use_share = stats.use_share();
            let free_share = 1.0 - use_share;
            sub
                .set("size", units.fmt(stats.size()))
                .set("used", units.fmt(stats.used()))
                .set("use-percents", format!("{:.0}%", 100.0 * use_share))
                .set_md("bar", progress_bar_md(use_share, BAR_WIDTH, args.ascii))
                .set("free", units.fmt(stats.available()))
                .set("free-percents", format!("{:.0}%", 100.0 * free_share));
            if let Some(inodes) = &stats.inodes {
                let iuse_share = inodes.use_share();
                sub
                    .set("inodes", inodes.files)
                    .set("iused", inodes.used())
                    .set("iuse-percents", format!("{:.0}%", 100.0 * iuse_share))
                    .set_md("ibar", progress_bar_md(iuse_share, INODES_BAR_WIDTH, args.ascii))
                    .set("ifree", inodes.favail);
            }
        } else if mount.is_unreachable() {
            sub.set("use-error", "unreachable");
        }
    }
    let mut skin = if color {
        make_colored_skin()
    } else {
        MadSkin::no_style()
    };
    if args.ascii {
        skin.limit_to_ascii();
    }

    let mut tbl = TableBuilder::default();
    for col in args.cols.cols() {
        tbl.col(
            minimad::Col::new(
                col.title(),
                match col {
                    Col::Id => "${id}",
                    Col::Dev => "${dev-major}:${dev-minor}",
                    Col::Filesystem => "${filesystem}",
                    Col::Label => "${label}",
                    Col::Disk => "${disk}",
                    Col::Type => "${type}",
                    Col::Remote => "${remote}",
                    Col::Used => "~~${used}~~",
                    Col::Use => "~~${use-percents}~~ ${bar}~~${use-error}~~",
                    Col::UsePercent => "~~${use-percents}~~",
                    Col::Free => "*${free}*",
                    Col::FreePercent => "*${free-percents}*",
                    Col::Size => "**${size}**",
                    Col::InodesFree => "*${ifree}*",
                    Col::InodesUsed => "~~${iused}~~",
                    Col::InodesUse => "~~${iuse-percents}~~ ${ibar}",
                    Col::InodesUsePercent => "~~${iuse-percents}~~",
                    Col::InodesCount => "**${inodes}**",
                    Col::MountPoint => "${mount-point}",
                    Col::Uuid => "${uuid}",
                    Col::PartUuid => "${part_uuid}",
                }
            )
            .align_content(col.content_align())
            .align_header(col.header_align())
        );
    }

    skin.print_owning_expander_md(&expander, &tbl);
}

fn make_colored_skin() -> MadSkin {
    MadSkin {
        bold: CompoundStyle::with_fg(AnsiValue(SIZE_COLOR)), // size
        inline_code: CompoundStyle::with_fgbg(AnsiValue(USED_COLOR), AnsiValue(AVAI_COLOR)), // use bar
        strikeout: CompoundStyle::with_fg(AnsiValue(USED_COLOR)), // use%
        italic: CompoundStyle::with_fg(AnsiValue(AVAI_COLOR)), // available
        ..Default::default()
    }
}

fn progress_bar_md(
    share: f64,
    bar_width: usize,
    ascii: bool,
) -> String {
    if ascii {
        let count = (share * bar_width as f64).round() as usize;
        let bar: String = "".repeat(count);
        let no_bar: String = "-".repeat(bar_width-count);
        format!("~~{}~~*{}*", bar, no_bar)
    } else {
        let pb = ProgressBar::new(share as f32, bar_width);
        format!("`{:<width$}`", pb, width = bar_width)
    }
}

