use std::collections::HashMap;
use std::any::{Any, TypeId};


pub struct Data {
	inner: HashMap<TypeId, Box<dyn Any + Send + Sync>>
}

impl Data {
	pub(crate) fn new() -> Self {
		Self {
			inner: HashMap::new()
		}
	}

	pub fn exists<D>(&self) -> bool
	where D: Any {
		self.inner.contains_key(&TypeId::of::<D>())
	}

	/// returns true if the data already existed
	pub(crate) fn insert<D>(&mut self, data: D) -> bool
	where D: Any + Send + Sync {
		self.inner.insert(data.type_id(), Box::new(data)).is_some()
	}

	pub fn get<D>(&self) -> Option<&D>
	where D: Any {
		self.inner.get(&TypeId::of::<D>())
			.and_then(|a| a.downcast_ref())
	}
}