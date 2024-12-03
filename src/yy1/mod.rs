use converter::YY1Converter;
use std::io;

mod converter;
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
    CN040,
    CN065,
    CN100,
    CN140,
    CN220,
    CN400,
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
    unit_width: f32,
    unit_length: f32,
    explode: bool,
}

impl PanelConfig {
    pub fn new(explode: bool, rows: u8, columns: u8, unit_width: f32, unit_length: f32) -> Self {
        Self {
            explode,
            rows,
            columns,
            unit_width,
            unit_length,
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
                self.unit_length, self.unit_width, self.rows, self.columns,
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
            unit_width: 0.0,
            unit_length: 0.0,
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

pub fn convert(
    input_path: String,
    output_path: String,
    feeder_config_path: Option<&String>,
    nozzle_config_path: Option<&String>,
    panel: Option<PanelConfig>,
    offset: Option<(f32, f32)>,
    fiducial_ref: Option<String>,
) -> io::Result<()> {
    let mut converter = YY1Converter::try_new(
        input_path,
        output_path,
        panel.unwrap_or_default(),
        fiducial_ref,
        feeder_config_path,
        nozzle_config_path,
    )?;
    converter.apply_offset(offset.unwrap_or((0.0, 0.0)));
    converter.panelize();
    converter.assign_nozzles();
    converter.write_files()
}
