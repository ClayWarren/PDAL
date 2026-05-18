use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

#[derive(Clone, Copy, Default)]
struct ChipPtRef {
    pos: f64,
    ptindex: PointId,
    oindex: usize,
}

pub struct ChipperFilter {
    threshold: PointId,
    partitions: Vec<usize>,
    outputs: Vec<PointView>,
    input: Option<PointView>,
}

impl ChipperFilter {
    pub fn new(threshold: PointId) -> Self {
        Self {
            threshold,
            partitions: Vec::new(),
            outputs: Vec::new(),
            input: None,
        }
    }

    fn load(view: &PointView) -> (Vec<ChipPtRef>, Vec<ChipPtRef>, Vec<ChipPtRef>) {
        let mut xvec = Vec::with_capacity(view.len() as usize);
        let mut yvec = Vec::with_capacity(view.len() as usize);
        for i in 0..view.len() {
            xvec.push(ChipPtRef {
                pos: view.get_f64(i, &DimId::X),
                ptindex: i,
                oindex: 0,
            });
            yvec.push(ChipPtRef {
                pos: view.get_f64(i, &DimId::Y),
                ptindex: i,
                oindex: 0,
            });
        }

        xvec.sort_by(|a, b| {
            a.pos
                .partial_cmp(&b.pos)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (i, x) in xvec.iter().enumerate() {
            yvec[x.ptindex as usize].oindex = i;
        }

        yvec.sort_by(|a, b| {
            a.pos
                .partial_cmp(&b.pos)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (i, y) in yvec.iter().enumerate() {
            xvec[y.oindex].oindex = i;
        }

        let spare = vec![ChipPtRef::default(); view.len() as usize];
        (xvec, yvec, spare)
    }

    fn partition(&mut self, size: usize) {
        let mut num_partitions = size / self.threshold as usize;
        if !size.is_multiple_of(self.threshold as usize) {
            num_partitions += 1;
        }
        let partition_size = size as f64 / num_partitions as f64;
        let mut total = 0.0;
        self.partitions.push(0);
        for _ in 0..num_partitions {
            total += partition_size;
            self.partitions.push(total.round() as usize);
        }
    }

    fn decide_split(
        &mut self,
        v1: &mut [ChipPtRef],
        v2: &mut [ChipPtRef],
        spare: &mut [ChipPtRef],
        pleft: usize,
        pright: usize,
    ) {
        let left = self.partitions[pleft];
        let right = self.partitions[pright] - 1;
        let v1range = v1[right].pos - v1[left].pos;
        let v2range = v2[right].pos - v2[left].pos;
        if v1range > v2range {
            self.split(v1, v2, spare, pleft, pright);
        } else {
            self.split(v2, v1, spare, pleft, pright);
        }
    }

    fn split(
        &mut self,
        wide: &mut [ChipPtRef],
        narrow: &mut [ChipPtRef],
        spare: &mut [ChipPtRef],
        pleft: usize,
        pright: usize,
    ) {
        let left = self.partitions[pleft];
        let right = self.partitions[pright] - 1;

        if pright - pleft == 1 {
            self.emit(wide, left, right);
        } else if pright - pleft == 2 {
            let center = self.partitions[pright - 1];
            self.emit(wide, left, center - 1);
            self.emit(wide, center, right);
        } else {
            let pcenter = (pleft + pright) / 2;
            let center = self.partitions[pcenter];
            let mut lstart = left;
            let mut rstart = center;
            for i in left..=right {
                if narrow[i].oindex < center {
                    spare[lstart] = narrow[i];
                    wide[narrow[i].oindex].oindex = lstart;
                    lstart += 1;
                } else {
                    spare[rstart] = narrow[i];
                    wide[narrow[i].oindex].oindex = rstart;
                    rstart += 1;
                }
            }
            self.decide_split(wide, spare, narrow, pleft, pcenter);
            self.decide_split(wide, spare, narrow, pcenter, pright);
        }
    }

    fn emit(&mut self, wide: &[ChipPtRef], widemin: usize, widemax: usize) {
        if let Some(input) = &self.input {
            let mut view = input.make_new();
            for chip in &wide[widemin..=widemax] {
                view.append_point(input, chip.ptindex);
            }
            self.outputs.push(view);
        }
    }
}

impl Filter for ChipperFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.chipper"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }
        self.partitions.clear();
        self.outputs.clear();
        self.input = Some(input.clone());

        let (mut xvec, mut yvec, mut spare) = Self::load(input);
        self.partition(xvec.len());
        self.decide_split(
            &mut xvec,
            &mut yvec,
            &mut spare,
            0,
            self.partitions.len() - 1,
        );
        self.input = None;
        Ok(std::mem::take(&mut self.outputs))
    }
}

impl Streamable for ChipperFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}
