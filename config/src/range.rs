use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ExperimentRange<T> {
	Fixed(T),
	List(Vec<T>),
	Range { start: T, end: T },
	RangeStride { start: T, end: T, stride: T },
}

impl<T> ExperimentRange<T> {
	pub fn is_variable(&self) -> bool {
		match self {
			Self::Fixed(_) => false,
			Self::List(l) => l.len() > 1,
			_ => true,
		}
	}
}

#[derive(Clone, Debug)]
pub struct ExperimentRangeIter<T> {
	range: ExperimentRange<T>,
	idx: usize,
}

impl Iterator for ExperimentRangeIter<u64> {
	type Item = u64;

	fn next(&mut self) -> Option<Self::Item> {
		let out = match &self.range {
			ExperimentRange::Fixed(v) if self.idx == 0 => Some(*v),
			ExperimentRange::List(l) => l.get(self.idx).copied(),
			ExperimentRange::Range { start: s, end: e } if e >= s && (self.idx as u64) <= e - s =>
				Some(s + self.idx as u64),
			ExperimentRange::RangeStride {
				start: s,
				end: e,
				stride,
			} if e >= s && (self.idx as u64) * stride <= e - s => Some(s + stride * (self.idx as u64)),
			_ => None,
		};

		if out.is_some() {
			self.idx += 1;
		}

		out
	}
}

impl Iterator for ExperimentRangeIter<f64> {
	type Item = f64;

	fn next(&mut self) -> Option<Self::Item> {
		let out = match &self.range {
			ExperimentRange::Fixed(v) if self.idx == 0 => Some(*v),
			ExperimentRange::List(l) => l.get(self.idx).copied(),
			ExperimentRange::Range { start: s, end: e } if e >= s && (self.idx as u64) <= 10 =>
				Some(s + (e - s) * (self.idx as f64 / 10.0)),
			ExperimentRange::RangeStride {
				start: s,
				end: e,
				stride,
			} if e >= s && (self.idx as f64) * stride <= e - s => Some(s + stride * (self.idx as f64)),
			_ => None,
		};

		if out.is_some() {
			self.idx += 1;
		}

		out
	}
}

impl<T> IntoIterator for ExperimentRange<T>
where
	ExperimentRangeIter<T>: Iterator<Item = T>,
{
	type Item = T;
	type IntoIter = ExperimentRangeIter<T>;

	fn into_iter(self) -> Self::IntoIter {
		ExperimentRangeIter {
			range: self,
			idx: 0,
		}
	}
}

impl<T> IntoIterator for &ExperimentRange<T>
where
	ExperimentRangeIter<T>: Iterator<Item = T>,
	ExperimentRange<T>: Clone,
{
	type Item = T;
	type IntoIter = ExperimentRangeIter<T>;

	fn into_iter(self) -> Self::IntoIter {
		ExperimentRangeIter {
			range: self.clone(),
			idx: 0,
		}
	}
}
