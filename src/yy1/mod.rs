use converter::YY1Converter;
use std::io;

mod converter;
mod package;
mod planner;

#[derive(Clone, Debug, serde::Serialize)]
pub struct ComponentRecord {
    #[serde(rename = "Designator")]
    reference: String,

    #[serde(rename = "Comment")]
    value: String,

    #[serde(rename = "Footprint")]
    package: String,

    #[serde(rename = "Mid X(mm)")]
    position_x: f32,

    #[serde(rename = "Mid Y(mm)")]
    position_y: f32,

    #[serde(rename = "Rotation")]
    rotation: f32,

    #[serde(rename = "Head")]
    head: u8,

    #[serde(rename = "FeederNo")]
    feeder: u8,

    #[serde(rename = "Mount Speed(%)")]
    mount_speed: u8,

    #[serde(rename = "Pick Height(mm)")]
    pick_height: f32,

    #[serde(rename = "Place Height(mm)")]
    place_height: f32,

    #[serde(rename = "Mode")]
    mode: u8,

    #[serde(rename = "Skip")]
    skip: u8,

    #[serde(skip)]
    nozzle: Option<Nozzle>,
}

impl ComponentRecord {
    pub fn placeholder() -> Self {
        Self {
            reference: "PH".into(),
            value: "Placeholder".into(),
            package: "Placeholder".into(),
            position_x: 0.0,
            position_y: 0.0,
            rotation: 0.0,
            head: 0,
            feeder: 0,
            mount_speed: 0,
            pick_height: 0.0,
            place_height: 0.0,
            mode: 0,
            skip: 1,
            nozzle: None,
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct KiCadRecord {
    #[serde(rename = "Ref")]
    reference: String,

    #[serde(rename = "Val")]
    value: String,

    #[serde(rename = "Package")]
    package: String,

    #[serde(rename = "PosX")]
    position_x: f32,

    #[serde(rename = "PosY")]
    position_y: f32,

    #[serde(rename = "Rot")]
    rotation: f32,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum Nozzle {
    /// 0201
    CN030,
    /// 0402
    CN040,
    /// 0402、0603
    CN065,
    /// 0805、Diodes、1206、1210
    CN100,
    /// 1206、1210、1812、2010、SOT23、5050
    CN140,
    /// SOP、SOT89、SOT223、SOT252
    CN220,
    /// ICs from 5 to 12mm
    CN400,
    /// ICs bigger than 12mm
    CN750,
    /// 3528serices Soft bead
    YX01,
    /// High power lamp beads
    YX02,
    /// Chips and BGA from 11mm to 17mm BGA
    YX03,
    /// Chips and BGA bigger than 17mm
    YX04,
    /// 4148 circular diode
    YX05,
    /// 3535 ball shape LED (Spherical height 1.4mm, overall height 1.9mm）
    YX06,
}

#[derive(Debug, serde::Deserialize)]
pub struct FeederConfig {
    #[serde(rename = "Feeder")]
    feeder: u8,

    #[serde(rename = "Package")]
    package: String,

    #[serde(rename = "Value")]
    value: String,

    #[serde(rename = "Rotation")]
    rotation: f32,

    #[serde(rename = "PickHeight")]
    pick_height: f32,

    #[serde(rename = "PlaceHeight")]
    place_height: f32,

    #[serde(rename = "MountSpeed")]
    mount_speed: u8,

    #[serde(rename = "Nozzle")]
    nozzle: Nozzle,

    #[serde(rename = "Mode")]
    mode: u8,

    #[serde(rename = "Skip")]
    skip: u8,
}

impl From<KiCadRecord> for ComponentRecord {
    fn from(value: KiCadRecord) -> Self {
        Self {
            package: value.package,
            reference: value.reference,
            value: value.value,
            position_x: value.position_x,
            position_y: value.position_y,
            rotation: value.rotation,
            head: 0,
            feeder: 0,
            mount_speed: 100,
            pick_height: 0.0,
            place_height: 0.0,
            mode: 0,
            skip: 0,
            nozzle: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PanelConfig {
    rows: u8,
    columns: u8,
    size: Size,
    explode: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn zero() -> Self {
        Self {
            width: 0.0,
            height: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Position {
    x: f32,
    y: f32,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

#[derive(Clone, Debug)]
pub enum Fiducial {
    Reference(String),
    Position(Position),
}

impl PanelConfig {
    pub fn new(explode: bool, rows: u8, columns: u8, size: Size) -> Self {
        Self {
            explode,
            rows,
            columns,
            size,
        }
    }

    pub fn explode(self, explode: bool) -> Self {
        Self { explode, ..self }
    }

    pub fn as_string(&self) -> String {
        if self.explode {
            "PanelizedPCB,UnitLength,0,UnitWidth,0,Rows,1,Columns,1,".into()
        } else {
            format!(
                "PanelizedPCB,UnitLength,{},UnitWidth,{},Rows,{},Columns,{},",
                self.size.height, self.size.width, self.rows, self.columns,
            )
        }
    }
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            explode: false,
            columns: 1,
            rows: 1,
            size: Size::default(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum NozzleStation {
    Station1,
    Station2,
    Station3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Head {
    Head1 = 1,
    Head2 = 2,
}

impl std::ops::Not for Head {
    type Output = Head;

    fn not(self) -> Self::Output {
        match self {
            Head::Head1 => Head::Head2,
            Head::Head2 => Head::Head1,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NozzleChange {
    enabled: bool,
    before_component: usize,
    head: Head,
    drop_station: NozzleStation,
    pickup_station: NozzleStation,
}

impl NozzleChange {
    pub fn as_string(&self) -> String {
        let state = if self.enabled { "ON" } else { "OFF" };
        format!(
            "NozzleChange,{},BeforeComponent,{},{:?},Drop,{:?},PickUp,{:?},",
            state, self.before_component, self.head, self.drop_station, self.pickup_station
        )
    }
}

impl Default for NozzleChange {
    fn default() -> Self {
        Self {
            enabled: false,
            before_component: 1,
            head: Head::Head1,
            drop_station: NozzleStation::Station3,
            pickup_station: NozzleStation::Station3,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PackageMap {
    #[serde(rename = "From")]
    from: String,

    #[serde(rename = "To")]
    to: String,
}
impl PackageMap {
    fn rename(&self, package: &str) -> Option<String> {
        if self.from == package {
            Some(self.to.clone())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    input_path: String,
    output_path: String,
    panel: PanelConfig,
    offset: Vec<Position>,
    feeder_config_path: Option<String>,
    nozzle_config_path: Option<String>,
    package_map_path: Option<String>,
    fiducial: Option<Fiducial>,
}

impl Config {
    pub fn new(input_path: String, output_path: String) -> Self {
        Self {
            input_path,
            output_path,
            feeder_config_path: None,
            nozzle_config_path: None,
            package_map_path: None,
            panel: PanelConfig::default(),
            offset: vec![Position::zero()],
            fiducial: None,
        }
    }

    pub fn panel(self, val: PanelConfig) -> Self {
        Self { panel: val, ..self }
    }

    pub fn offset(self, val: Vec<Position>) -> Self {
        Self {
            offset: val,
            ..self
        }
    }

    pub fn feeder_config_path(self, val: Option<String>) -> Self {
        Self {
            feeder_config_path: val,
            ..self
        }
    }

    pub fn nozzle_config_path(self, val: Option<String>) -> Self {
        Self {
            nozzle_config_path: val,
            ..self
        }
    }

    pub fn package_map_path(self, val: Option<String>) -> Self {
        Self {
            package_map_path: val,
            ..self
        }
    }

    pub fn fiducial(self, val: Option<Fiducial>) -> Self {
        Self {
            fiducial: val,
            ..self
        }
    }
}

pub fn convert(config: Config) -> io::Result<()> {
    let mut converter = YY1Converter::try_new(config)?;
    converter.panelize();
    converter.apply_offset();
    converter.assign_nozzles();
    converter.write_files()
}
