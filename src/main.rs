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
                .help("PCB offset (x:y)"),
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
            Arg::new("bom")
                .long("bom")
                .short('b')
                .num_args(0)
                .help("Generate BOM"),
        )
        .arg(
            Arg::new("skip")
                .short('s')
                .long("skip")
                .help("Skip until component number"),
        )
        .arg(
            Arg::new("fiducial")
                .allow_hyphen_values(true)
                .long("fiducial")
                .help("Fiducial designator or position"),
        )
}

fn parse_offset(offset: &str) -> Result<Position, String> {
    let (x, y) = offset.split_once(':').ok_or("Invalid offset config")?;
    Ok(Position::new(
        x.parse().map_err(|_| "Invalid X offset")?,
        y.parse().map_err(|_| "Invalid Y offset")?,
    ))
}

fn parse_fiducial(fudicial: &str) -> Result<Fiducial, String> {
    if fudicial.contains(':') {
        let (x, y) = fudicial
            .split_once(':')
            .ok_or("Invalid fiducial position")?;
        Ok(Fiducial::Position(Position::new(
            x.parse().map_err(|_| "Invalid fiducial X position")?,
            y.parse().map_err(|_| "Invalid fiducial Y position")?,
        )))
    } else {
        Ok(Fiducial::Reference(fudicial.into()))
    }
}

fn parse_panel_config(panel: &str) -> Result<PanelConfig, String> {
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
            Ok(PanelConfig::new(
                false,
                columns,
                rows,
                Size::new(width, length),
            ))
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
                .collect::<Result<Vec<Position>, String>>()
        })
        .transpose()
        .map_err(io::Error::other)?
        .unwrap_or(vec![Position::zero()]);
    let panel = matches
        .get_one::<String>("panel")
        .map(|panel| {
            parse_panel_config(panel).map(|panel| panel.explode(matches.get_flag("explode")))
        })
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
    .fiducial(
        matches
            .get_one::<String>("fiducial")
            .map(|fiducial| parse_fiducial(fiducial))
            .transpose()
            .map_err(io::Error::other)?,
    )
    .skip_until(
        matches
            .get_one::<String>("skip")
            .map(|val| val.parse())
            .transpose()
            .map_err(io::Error::other)?,
    )
    .panel(panel)
    .bom(matches.get_flag("bom"))
    .offset(offset);

    convert(config)
}
