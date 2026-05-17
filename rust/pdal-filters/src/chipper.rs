use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

#[derive(Clone, Copy, Default)]
struct ChipRef {
    pos: f64,
    point_index: PointId,
    other_index: usize,
}

pub struct ChipperFilter {
    capacity: PointId,
}

impl ChipperFilter {
    pub fn new(capacity: PointId) -> Self {
        Self { capacity }
    }
}

impl Filter for ChipperFilter {
    fn name(&self) -> &str {
        "filters.chipper"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        if view.is_empty() {
            return Ok(Vec::new());
        }

        let mut x_refs = Vec::with_capacity(view.len() as usize);
        let mut y_refs = Vec::with_capacity(view.len() as usize);
        let mut spare = vec![ChipRef::default(); view.len() as usize];
        load(view, &mut x_refs, &mut y_refs);
        let partitions = partitions(view.len(), self.capacity.max(1));
        let mut chips = Vec::new();
        decide_split(
            view,
            &partitions,
            &mut chips,
            &mut x_refs,
            &mut y_refs,
            &mut spare,
            0,
            partitions.len() - 1,
        );
        Ok(chips)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for ChipperFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

fn load(view: &PointView, x_refs: &mut Vec<ChipRef>, y_refs: &mut Vec<ChipRef>) {
    for i in 0..view.len() {
        x_refs.push(ChipRef {
            pos: view.get_f64(i, &DimId::X),
            point_index: i,
            other_index: 0,
        });
        y_refs.push(ChipRef {
            pos: view.get_f64(i, &DimId::Y),
            point_index: i,
            other_index: 0,
        });
    }

    x_refs.sort_by(|left, right| left.pos.total_cmp(&right.pos));
    for (i, x_ref) in x_refs.iter().enumerate() {
        y_refs[x_ref.point_index as usize].other_index = i;
    }

    y_refs.sort_by(|left, right| left.pos.total_cmp(&right.pos));
    for (i, y_ref) in y_refs.iter().enumerate() {
        x_refs[y_ref.other_index].other_index = i;
    }
}

fn partitions(size: PointId, capacity: PointId) -> Vec<PointId> {
    let mut partition_count = size / capacity;
    if !size.is_multiple_of(capacity) {
        partition_count += 1;
    }

    let partition_size = size as f64 / partition_count as f64;
    let mut total = 0.0;
    let mut out = vec![0];
    for _ in 0..partition_count {
        total += partition_size;
        out.push(total.round() as PointId);
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn decide_split(
    input: &PointView,
    partitions: &[PointId],
    chips: &mut Vec<PointView>,
    refs1: &mut [ChipRef],
    refs2: &mut [ChipRef],
    spare: &mut [ChipRef],
    partition_left: usize,
    partition_right: usize,
) {
    let left = partitions[partition_left] as usize;
    let right = partitions[partition_right] as usize - 1;
    let range1 = refs1[right].pos - refs1[left].pos;
    let range2 = refs2[right].pos - refs2[left].pos;
    if range1 > range2 {
        split(
            input,
            partitions,
            chips,
            refs1,
            refs2,
            spare,
            partition_left,
            partition_right,
        );
    } else {
        split(
            input,
            partitions,
            chips,
            refs2,
            refs1,
            spare,
            partition_left,
            partition_right,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn split(
    input: &PointView,
    partitions: &[PointId],
    chips: &mut Vec<PointView>,
    wide: &mut [ChipRef],
    narrow: &mut [ChipRef],
    spare: &mut [ChipRef],
    partition_left: usize,
    partition_right: usize,
) {
    let left = partitions[partition_left] as usize;
    let right = partitions[partition_right] as usize - 1;

    if partition_right - partition_left == 1 {
        emit(input, chips, wide, left, right);
    } else if partition_right - partition_left == 2 {
        let center = partitions[partition_right - 1] as usize;
        emit(input, chips, wide, left, center - 1);
        emit(input, chips, wide, center, right);
    } else {
        let partition_center = (partition_left + partition_right) / 2;
        let center = partitions[partition_center] as usize;
        let mut left_start = left;
        let mut right_start = center;
        for i in left..=right {
            if narrow[i].other_index < center {
                spare[left_start] = narrow[i];
                wide[narrow[i].other_index].other_index = left_start;
                left_start += 1;
            } else {
                spare[right_start] = narrow[i];
                wide[narrow[i].other_index].other_index = right_start;
                right_start += 1;
            }
        }

        decide_split(
            input,
            partitions,
            chips,
            wide,
            spare,
            narrow,
            partition_left,
            partition_center,
        );
        decide_split(
            input,
            partitions,
            chips,
            wide,
            spare,
            narrow,
            partition_center,
            partition_right,
        );
    }
}

fn emit(input: &PointView, chips: &mut Vec<PointView>, refs: &[ChipRef], min: usize, max: usize) {
    let mut chip = input.make_new();
    for item in refs.iter().take(max + 1).skip(min) {
        chip.append_point(input, item.point_index);
    }
    chips.push(chip);
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    #[test]
    fn partitions_points_by_capacity() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for i in 0..10 {
            let id = view.add_point();
            view.set_f64(id, &DimId::X, i as f64);
            view.set_f64(id, &DimId::Y, 0.0);
        }

        let mut filter = ChipperFilter::new(3);
        let chips = filter.run(&view).unwrap();

        assert_eq!(chips.len(), 4);
        assert_eq!(chips.iter().map(PointView::len).sum::<u64>(), 10);
        assert!(chips.iter().all(|chip| chip.len() <= 3));
    }
}
