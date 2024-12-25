use super::*;
use package::PackageConverter;
use planner::*;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

pub struct YY1Converter {
    fiducial: (f32, f32),
    config: Config,
    steps: Vec<PickAndPlaceStep>,
}

impl YY1Converter {
    pub fn try_new(config: Config) -> io::Result<Self> {
        let mut reader = csv::Reader::from_reader(File::open(&config.input_path)?);
        let kicad_records: Result<Vec<KiCadRecord>, csv::Error> = reader.deserialize().collect();

        let feeder_config: Option<HashMap<(String, String), FeederConfig>> =
            if let Some(path) = &config.feeder_config_path {
                let mut reader = csv::Reader::from_reader(File::open(path)?);
                let records: Result<Vec<FeederConfig>, csv::Error> = reader.deserialize().collect();
                let feeder_config = records
                    .map_err(|err| io::Error::other(err.to_string()))?
                    .into_iter()
                    .map(|cfg| ((cfg.value.clone(), cfg.package.clone()), cfg))
                    .collect();
                Some(feeder_config)
            } else {
                None
            };

        let nozzles_config = match &config.nozzle_config_path {
            Some(path) => {
                let mut reader = csv::Reader::from_reader(File::open(path)?);
                let records: Result<Vec<NozzleConfig>, csv::Error> = reader.deserialize().collect();
                records?.into_iter().map(Option::Some).collect()
            }
            None => vec![None],
        };

        let package_map = match &config.package_map_path {
            Some(path) => {
                let mut reader = csv::Reader::from_reader(File::open(path)?);
                let records: Result<Vec<PackageMap>, csv::Error> = reader.deserialize().collect();
                records?.into_iter().collect()
            }
            None => vec![],
        };
        let package_converter = PackageConverter::new(package_map);

        let components: Vec<ComponentRecord> = kicad_records
            .map_err(|err| io::Error::other(err.to_string()))?
            .into_iter()
            .map(|comp| {
                let mut comp: ComponentRecord = comp.into();
                comp.package = package_converter.rename(&comp.package);

                if comp.value == "Fiducial" {
                    comp.skip = 1;
                }

                if let Some(feeder_config) = &feeder_config {
                    let comp_kind = (comp.value.clone(), comp.package.clone());
                    if let Some(feeder) = feeder_config.get(&comp_kind) {
                        comp.feeder = feeder.feeder;
                        comp.pick_height = feeder.pick_height;
                        comp.place_height = feeder.place_height;
                        comp.mount_speed = feeder.mount_speed;
                        comp.mode = feeder.mode;
                        comp.skip = feeder.skip;
                        comp.rotation = match (comp.rotation + feeder.rotation) % 360.0 {
                            -0.0 => 0.0,
                            angle if angle <= -180.0 => angle + 360.0,
                            angle if angle > 180.0 => angle - 360.0,
                            angle => angle,
                        };
                        for nozzle_config in &nozzles_config {
                            if nozzle_config
                                .map(|cfg| cfg.contains(feeder.nozzle))
                                .unwrap_or(false)
                            {
                                comp.nozzle = Some(feeder.nozzle);
                                break;
                            }
                        }
                        if comp.nozzle.is_none() && nozzles_config.iter().any(Option::is_some) {
                            comp.skip = 1;
                            eprintln!(
                                "Warning: Nozzle {:?} not found for component: {} {}. Skipping...",
                                feeder.nozzle, comp.value, comp.package
                            );
                        }
                    } else if comp.value != "Fiducial" {
                        comp.skip = 1;
                        eprintln!(
                            "Warning: Feeder not found for component: {} - {}. Skipping...",
                            comp.value, comp.package
                        );
                    }
                }

                comp
            })
            .collect();

        let fiducial = match &config.fiducial_ref {
            Some(fiducial_ref) => components
                .iter()
                .find(|fid| fid.reference == *fiducial_ref)
                .map(|fid| {
                    let (columns, rows) = if config.panel.explode {
                        (
                            (config.panel.columns - 1) as f32,
                            (config.panel.rows - 1) as f32,
                        )
                    } else {
                        (0.0, 0.0)
                    };
                    (
                        fid.position_x + columns * config.panel.unit_width,
                        fid.position_y + rows * config.panel.unit_length,
                    )
                })
                .ok_or(io::Error::other("Fiducial not found"))?,
            None => (0.0, 0.0),
        };

        let multi_step = nozzles_config.len() > 1;
        let output_path = Path::new(&config.output_path);
        let mut steps: Vec<PickAndPlaceStep> = nozzles_config
            .into_iter()
            .enumerate()
            .map(|(idx, nozzle_config)| {
                let file_name = output_path
                    .file_stem()
                    .map(|step| {
                        if multi_step {
                            format!("{}_{}", step.to_string_lossy(), idx + 1)
                        } else {
                            step.to_string_lossy().into_owned()
                        }
                    })
                    .unwrap();
                let file_path = output_path
                    .with_file_name(file_name)
                    .with_extension("csv")
                    .to_string_lossy()
                    .into();
                PickAndPlaceStep {
                    nozzle_config,
                    file_path,
                    components: vec![],
                    nozzle_change: vec![],
                }
            })
            .collect();

        for comp in components.iter().filter(|comp| comp.skip == 0) {
            if let Some(nozzle) = comp.nozzle {
                if let Some(step) = steps.iter_mut().find(|step| {
                    step.nozzle_config
                        .map(|cfg| cfg.contains(nozzle))
                        .unwrap_or(false)
                }) {
                    step.components.push(comp.clone());
                }
            } else if let Some(step) = steps.first_mut() {
                step.components.push(comp.clone());
            }
        }

        Ok(Self {
            fiducial,
            config,
            steps,
        })
    }

    pub fn apply_offset(&mut self) {
        self.fiducial = (
            self.fiducial.0 + self.config.offset.0,
            self.fiducial.1 + self.config.offset.1,
        );
        for step in self.steps.iter_mut() {
            for component in step.components.iter_mut() {
                component.position_x += self.config.offset.0;
                component.position_y += self.config.offset.1;
            }
        }
    }

    pub fn panelize(&mut self) {
        if !self.config.panel.explode {
            return;
        }
        for step in self.steps.iter_mut() {
            let components = step.components.clone();
            step.components.clear();
            for col in 0..self.config.panel.columns {
                for row in 0..self.config.panel.rows {
                    for mut component in components.iter().cloned() {
                        let delta_x = col as f32 * self.config.panel.unit_width;
                        let delta_y = row as f32 * self.config.panel.unit_length;
                        component.position_x += delta_x;
                        component.position_y += delta_y;
                        component.reference =
                            format!("{0}_{1}_{2}", component.reference, col + 1, row + 1);
                        step.components.push(component);
                    }
                }
            }
        }
    }

    pub fn assign_nozzles(&mut self) {
        for step in self.steps.iter_mut() {
            step.assign_nozzles();
        }
    }

    pub fn write_files(&self) -> io::Result<()> {
        for step in &self.steps {
            let mut writer = File::create(&step.file_path)?;
            let mut nozzle_change = step.nozzle_change.iter().cloned();
            let header = format!(
                include_str!("header.csv"),
                self.config.panel.as_string(),
                self.fiducial.0,
                self.fiducial.1,
                nozzle_change.next().unwrap_or_default().as_string(),
                nozzle_change.next().unwrap_or_default().as_string(),
                nozzle_change.next().unwrap_or_default().as_string(),
                nozzle_change.next().unwrap_or_default().as_string()
            )
            .replace("\n", "\r\n");
            write!(&mut writer, "{header}")?;

            let mut csv_writer = csv::WriterBuilder::default()
                .terminator(csv::Terminator::CRLF)
                .from_writer(writer);
            for component in &step.components {
                csv_writer.serialize(component)?
            }
            csv_writer.flush()?;
        }

        Ok(())
    }
}

pub struct PickAndPlaceStep {
    file_path: String,
    components: Vec<ComponentRecord>,
    nozzle_change: Vec<NozzleChange>,
    nozzle_config: Option<NozzleConfig>,
}

impl PickAndPlaceStep {
    pub fn assign_nozzles(&mut self) {
        if let Some(nozzle_config) = self.nozzle_config {
            self.components.sort_by(|comp1, comp2| {
                let nozzle1 = comp1.nozzle.unwrap_or(Nozzle::CN040);
                let nozzle2 = comp2.nozzle.unwrap_or(Nozzle::CN040);
                match nozzle_config
                    .is_active(nozzle2)
                    .cmp(&nozzle_config.is_active(nozzle1))
                {
                    Ordering::Equal => match comp1.place_height.total_cmp(&comp2.place_height) {
                        Ordering::Equal => match nozzle1.cmp(&nozzle2) {
                            Ordering::Equal => comp1.feeder.cmp(&comp2.feeder),
                            ord => ord,
                        },
                        ord => ord,
                    },
                    ord => ord,
                }
            });

            let mut planner = Planner::new(nozzle_config, &self.components);
            for component in self.components.iter_mut() {
                if let Some(nozzle) = component.nozzle {
                    loop {
                        match planner.plan(nozzle) {
                            PlannerAction::Head(head) => {
                                component.head = head as _;
                                break;
                            }
                            PlannerAction::NozzleChange(nozzle_change) => {
                                self.nozzle_change.push(nozzle_change)
                            }
                        }
                    }
                }
            }

            let revert_nozzles = planner.finalize();
            if !revert_nozzles.is_empty() {
                self.nozzle_change.extend(revert_nozzles);
                self.components.push(ComponentRecord::placeholder());
            }
        }
    }
}
