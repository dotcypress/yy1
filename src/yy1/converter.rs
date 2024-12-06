use super::planner::*;
use super::*;
use regex::*;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};
use std::path::*;

pub struct YY1Converter {
    fiducial: (f32, f32),
    panel: PanelConfig,
    steps: Vec<PickAndPlaceStep>,
}

impl YY1Converter {
    pub fn try_new(
        input_path: String,
        output_path: String,
        panel: PanelConfig,
        fiducial_ref: Option<String>,
        feeder_config_path: Option<&String>,
        nozzles_config_path: Option<&String>,
    ) -> io::Result<Self> {
        let mut reader = csv::Reader::from_reader(File::open(input_path)?);
        let kicad_records: Result<Vec<KiCadRecord>, csv::Error> = reader.deserialize().collect();

        let feeder_config: Option<HashMap<(String, String), FeederConfig>> =
            if let Some(path) = feeder_config_path {
                let mut reader = csv::Reader::from_reader(File::open(path)?);
                let records: Result<Vec<FeederConfig>, csv::Error> = reader.deserialize().collect();
                let config = records
                    .map_err(|err| io::Error::other(err.to_string()))?
                    .into_iter()
                    .map(|cfg| ((cfg.value.clone(), cfg.package.clone()), cfg))
                    .collect();
                Some(config)
            } else {
                None
            };

        let nozzles_config = match nozzles_config_path {
            Some(path) => {
                let mut reader = csv::Reader::from_reader(File::open(path)?);
                let records: Result<Vec<NozzleConfig>, csv::Error> = reader.deserialize().collect();
                records?.into_iter().map(Option::Some).collect()
            }
            None => vec![None],
        };

        let package_converters = [
            (
                Regex::new(r"Crystal_SMD_([0-9]+)[_-].+").unwrap(),
                r"XTAL-${1}",
            ),
            (
                Regex::new(r"([VWDLTQ]?)F([NP]?)-([0-9]+)[-_].+").unwrap(),
                r"${1}F${2}-${3}",
            ),
            (Regex::new(r"LED_([0-9]+)_.+").unwrap(), r"${1}"),
            (Regex::new(r"[RCLD]_([0-9]+)_.+").unwrap(), r"${1}"),
        ];

        let components: Vec<ComponentRecord> = kicad_records
            .map_err(|err| io::Error::other(err.to_string()))?
            .into_iter()
            .map(|comp| {
                let mut comp: ComponentRecord = comp.into();
                for (re, replace) in &package_converters {
                    if re.is_match(&comp.package) {
                        comp.package = re.replace(&comp.package, *replace).into();
                    }
                }

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
                        comp.rotation = (comp.rotation + feeder.rotation) % 180.0;
                        comp.mode = feeder.mode;
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

        let fiducial = match fiducial_ref {
            Some(fiducial_ref) => components
                .iter()
                .find(|rec| rec.reference == fiducial_ref)
                .map(|rec| {
                    let (columns, rows) = if panel.explode {
                        ((panel.columns - 1) as f32, (panel.rows - 1) as f32)
                    } else {
                        (0.0, 0.0)
                    };
                    (
                        rec.position_x + columns * panel.unit_width,
                        rec.position_y + rows * panel.unit_length,
                    )
                })
                .ok_or(io::Error::other("Fiducial not found"))?,
            None => (0.0, 0.0),
        };

        let multi_step = nozzles_config.len() > 1;
        let output_path = Path::new(&output_path);
        let mut steps: Vec<PickAndPlaceStep> = nozzles_config
            .into_iter()
            .enumerate()
            .map(|(idx, nozzle_config)| {
                let file_name = output_path
                    .file_stem()
                    .map(|stem| {
                        if multi_step {
                            format!("{}_{}", stem.to_string_lossy(), idx + 1)
                        } else {
                            stem.to_string_lossy().into_owned()
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
            panel,
            steps,
        })
    }

    pub fn apply_offset(&mut self, offset: (f32, f32)) {
        self.fiducial = (self.fiducial.0 + offset.0, self.fiducial.1 + offset.1);
        for step in self.steps.iter_mut() {
            for component in step.components.iter_mut() {
                component.position_x += offset.0;
                component.position_y += offset.1;
            }
        }
    }

    pub fn panelize(&mut self) {
        if !self.panel.explode {
            return;
        }
        for step in self.steps.iter_mut() {
            let components = step.components.clone();
            step.components.clear();
            for col in 0..self.panel.columns {
                for row in 0..self.panel.rows {
                    for mut component in components.iter().cloned() {
                        let delta_x = col as f32 * self.panel.unit_width;
                        let delta_y = row as f32 * self.panel.unit_length;
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
                self.panel.as_string(),
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
                let nozzle1 = comp1.nozzle.unwrap();
                let nozzle2 = comp2.nozzle.unwrap();
                match nozzle_config
                    .is_active(nozzle2)
                    .cmp(&nozzle_config.is_active(nozzle1))
                {
                    Ordering::Equal => match nozzle1.cmp(&nozzle2) {
                        Ordering::Equal => comp1.feeder.cmp(&comp2.feeder),
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
