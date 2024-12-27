use clap::{arg, Arg, Command};
use std::io;
use yy1::*;

mod yy1;

fn cli() -> Command {
    Command::new("yy1")
        .about(
            "Utility to convert KiCad centroid files into Neoden YY1 pick and place machine format",
        )
        .arg_required_else_help(true)
        .arg(
            arg!(input: [INPUT])
                .help("KiCad placement file")
                .required(true),
        )
        .arg(arg!(output: [OUTPUT]).help("Output file(s)").required(true))
        .arg(
            Arg::new("package_map")
                .long("rename")
                .short('r')
                .help("Package rename file"),
        )
        .arg(
            Arg::new("feeder_config")
                .long("feeder")
                .short('f')
                .help("Feeder config file"),
        )
        .arg(
            Arg::new("nozzle_config")
                .long("nozzle")
                .short('n')
                .requires("feeder_config")
                .help("Nozzle config file"),
        )
        .arg(
            Arg::new("offset")
                .allow_hyphen_values(true)
                .value_delimiter(',')
                .long("offset")
                .short('o')
                .help("PCB offset"),
        )
        .arg(
            Arg::new("panel")
                .long("panel")
                .short('p')
                .help("Panel config (rows:columns:width:length)"),
        )
        .arg(
            Arg::new("explode")
                .long("explode")
                .short('e')
                .num_args(0)
                .help("Explode panel"),
        )
        .arg(
            Arg::new("fiducial")
                .long("fiducial")
                .help("Fiducial designator"),
        )
}

fn parse_offset(offset: &str) -> Result<(f32, f32), String> {
    let (x, y) = offset.split_once(':').ok_or("Invalid offset config")?;
    Ok((
        x.parse().map_err(|_| "Invalid X offset")?,
        y.parse().map_err(|_| "Invalid Y offset")?,
    ))
}

fn parse_panel(panel: &str) -> Result<PanelConfig, String> {
    let params: Vec<&str> = panel.split(':').collect();
    if params.len() != 4 {
        Err("Invalid panel config".into())
    } else {
        let rows = params[0].parse().map_err(|_| "Invalid panel rows")?;
        let columns = params[1].parse().map_err(|_| "Invalid panel columns")?;
        let width = params[2].parse().map_err(|_| "Invalid panel unit width")?;
        let length = params[3].parse().map_err(|_| "Invalid panel unit length")?;

        if columns < 2 || rows < 2 {
            Err("Invalid panel config".into())
        } else {
            Ok(PanelConfig::new(false, columns, rows, width, length))
        }
    }
}

fn main() -> io::Result<()> {
    let matches = cli().get_matches();
    let offset = matches
        .get_many::<String>("offset")
        .map(|offsets| {
            offsets
                .map(|offset| parse_offset(offset))
                .collect::<Result<Vec<(f32, f32)>, String>>()
        })
        .transpose()
        .map_err(io::Error::other)?
        .unwrap_or_default();
    let panel = matches
        .get_one::<String>("panel")
        .map(|panel| parse_panel(panel).map(|panel| panel.explode(matches.get_flag("explode"))))
        .transpose()
        .map_err(io::Error::other)?
        .unwrap_or_default();

    let config = Config::new(
        matches
            .get_one::<String>("input")
            .expect("required")
            .to_owned(),
        matches
            .get_one::<String>("output")
            .expect("required")
            .to_owned(),
    )
    .feeder_config_path(matches.get_one::<String>("feeder_config").cloned())
    .nozzle_config_path(matches.get_one::<String>("nozzle_config").cloned())
    .package_map_path(matches.get_one::<String>("package_map").cloned())
    .fiducial_ref(matches.get_one::<String>("fiducial").cloned())
    .panel(panel)
    .offset(offset);

    convert(config)
}
