use std::collections::HashMap;

use super::*;

#[derive(Debug, Clone, Copy, Default, serde::Deserialize)]
pub struct NozzleConfig {
    #[serde(rename = "Head1")]
    head1: Option<Nozzle>,

    #[serde(rename = "Head2")]
    head2: Option<Nozzle>,

    #[serde(rename = "Station1")]
    station1: Option<Nozzle>,

    #[serde(rename = "Station2")]
    station2: Option<Nozzle>,

    #[serde(skip)]
    station3: Option<Nozzle>,
}

impl NozzleConfig {
    pub fn contains(&self, nozzle: Nozzle) -> bool {
        let nozzle = Some(nozzle);
        self.head1 == nozzle
            || self.head2 == nozzle
            || self.station1 == nozzle
            || self.station2 == nozzle
    }

    pub fn is_active(&self, nozzle: Nozzle) -> bool {
        self.find_nozzle_head(nozzle).is_some()
    }

    pub fn get_head_nozzle(&self, head: Head) -> Option<Nozzle> {
        match head {
            Head::Head1 => self.head1,
            Head::Head2 => self.head2,
        }
    }

    pub fn find_nozzle_head(&self, nozzle: Nozzle) -> Option<Head> {
        let nozzle = Some(nozzle);
        if self.head1 == nozzle {
            Some(Head::Head1)
        } else if self.head2 == nozzle {
            Some(Head::Head2)
        } else {
            None
        }
    }

    pub fn find_nozzle_station(&self, nozzle: Nozzle) -> Option<NozzleStation> {
        let nozzle = Some(nozzle);
        if self.station1 == nozzle {
            Some(NozzleStation::Station1)
        } else if self.station2 == nozzle {
            Some(NozzleStation::Station2)
        } else if self.station3 == nozzle {
            Some(NozzleStation::Station3)
        } else {
            None
        }
    }

    pub fn drop_nozzle(&mut self, nozzle: Option<Nozzle>) -> NozzleStation {
        if self.station1.is_none() {
            self.station1 = nozzle;
            NozzleStation::Station1
        } else if self.station2.is_none() {
            self.station2 = nozzle;
            NozzleStation::Station2
        } else if self.station3.is_none() {
            self.station3 = nozzle;
            NozzleStation::Station3
        } else {
            unreachable!()
        }
    }

    pub fn pickup_nozzle(
        &mut self,
        head: Head,
        new_nozzle: Nozzle,
        before_component: usize,
    ) -> NozzleChange {
        let current_nozzle = match head {
            Head::Head1 => self.head1.replace(new_nozzle),
            Head::Head2 => self.head2.replace(new_nozzle),
        };
        let pickup_station = self.find_nozzle_station(new_nozzle).unwrap();
        let drop_station = self.drop_nozzle(current_nozzle);
        match pickup_station {
            NozzleStation::Station1 => self.station1 = None,
            NozzleStation::Station2 => self.station2 = None,
            NozzleStation::Station3 => self.station3 = None,
        }
        NozzleChange {
            head,
            drop_station,
            pickup_station,
            before_component,
            enabled: true,
        }
    }
}

#[derive(Debug)]
pub enum PlannerAction {
    Head(Head),
    NozzleChange(NozzleChange),
}

#[derive(Debug)]
pub struct Planner {
    component_index: usize,
    head: Head,
    config: NozzleConfig,
    nozzle_spans: HashMap<Nozzle, usize>,
    nozzle_history: Vec<NozzleChange>,
}

impl Planner {
    pub fn new(config: NozzleConfig, components: &[ComponentRecord]) -> Self {
        let nozzle_seq: Vec<Nozzle> = components.iter().filter_map(|comp| comp.nozzle).collect();
        let nozzle_spans =
            nozzle_seq
                .into_iter()
                .enumerate()
                .fold(HashMap::new(), |mut acc, (idx, nozzle)| {
                    acc.insert(nozzle, idx + 1);
                    acc
                });

        Self {
            config,
            nozzle_spans,
            component_index: 1,
            nozzle_history: vec![],
            head: Head::Head1,
        }
    }

    pub fn finalize(self) -> Vec<NozzleChange> {
        assert!(self.nozzle_history.len() <= 4);
        self.nozzle_history
            .into_iter()
            .rev()
            .map(|nozzle_change| NozzleChange {
                before_component: self.component_index,
                pickup_station: nozzle_change.drop_station,
                drop_station: nozzle_change.pickup_station,
                ..nozzle_change
            })
            .collect()
    }

    pub fn plan(&mut self, nozzle: Nozzle) -> PlannerAction {
        let active_nozzle = self.config.get_head_nozzle(self.head);
        let nozzle_expired = active_nozzle.is_none()
            || self
                .nozzle_spans
                .get(&active_nozzle.unwrap())
                .map(|ttl| *ttl < self.component_index)
                .unwrap_or_default();

        if nozzle_expired && self.config.find_nozzle_station(nozzle).is_some() {
            let nozzle_change = self
                .config
                .pickup_nozzle(self.head, nozzle, self.component_index);
            self.nozzle_history.push(nozzle_change);
            return PlannerAction::NozzleChange(nozzle_change);
        }

        if active_nozzle == Some(nozzle) {
            let action = PlannerAction::Head(self.head);
            self.component_index += 1;
            self.head = !self.head;
            action
        } else if self.config.get_head_nozzle(!self.head) == Some(nozzle) {
            self.component_index += 1;
            PlannerAction::Head(!self.head)
        } else {
            unreachable!()
        }
    }
}
