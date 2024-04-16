use std::{
	any::{Any, TypeId},
	collections::HashMap,
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

#[cfg(test)]
mod tests {
	use super::*;

	// test if we can store a reference
	struct WithRef<'a> {
		inner: &'a str,
	}

	// #[test]
	// fn test() {
	// 	let mut state = State::new();
	// 	let loc = String::from("123");
	// 	state.insert(WithRef { inner: &loc });

	// 	let with_ref = state.get::<WithRef>().unwrap();
	// 	assert_eq!(with_ref.inner, "hello");
	// }
}
