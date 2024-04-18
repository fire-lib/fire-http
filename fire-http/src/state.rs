use std::{
	any::{Any, TypeId},
	collections::{HashMap, HashSet},
};

pub struct State {
	inner: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl State {
	pub fn new() -> Self {
		Self {
			inner: HashMap::new(),
		}
	}

	pub fn insert<T>(&mut self, data: T)
	where
		T: Any + Send + Sync,
	{
		self.inner.insert(TypeId::of::<T>(), Box::new(data));
	}

	pub fn get<T>(&self) -> Option<&T>
	where
		T: Any + Send + Sync,
	{
		self.inner
			.get(&TypeId::of::<T>())
			.map(|b| b.downcast_ref::<T>().unwrap())
	}
}

pub struct StateValidation {
	inner: HashSet<TypeId>,
}

impl StateValidation {
	pub fn new() -> Self {
		Self {
			inner: HashSet::new(),
		}
	}

	pub fn insert<T>(&mut self)
	where
		T: Any + Send + Sync,
	{
		self.inner.insert(TypeId::of::<T>());
	}

	pub fn validate<T>(&self) -> bool
	where
		T: Any + Send + Sync,
	{
		self.inner.contains(&TypeId::of::<T>())
	}
}
