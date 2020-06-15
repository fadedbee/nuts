use crate::nut::iac::{filter::SubscriptionFilter, managed_state::DomainId, topic::Topic};
use crate::nut::Handler;
use std::any::Any;
use std::collections::HashMap;
use std::ops::{Index, IndexMut};

pub trait Activity: Any {}
impl<T: Any> Activity for T {}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct ActivityId<A> {
    pub(crate) index: usize,
    pub(crate) domain_index: DomainId,
    phantom: std::marker::PhantomData<A>,
}

/// A collection of heterogenous Activities
///
/// Needs stores a list of dynamic `Any` trait objects, not `Activity` because
/// trait objects only allow access to methods of that trait, not their super-traits.  
#[derive(Default)]
pub(crate) struct ActivityContainer {
    data: Vec<Option<Box<dyn Any>>>,
    active: Vec<bool>,
}

/// Handlers stored per Activity
#[derive(Default)]
pub(crate) struct ActivityHandlerContainer {
    data: HashMap<usize, Vec<Handler>>,
}

impl<A: Activity> ActivityId<A> {
    pub(crate) fn new(index: usize, domain_index: DomainId) -> Self {
        Self {
            index,
            domain_index,
            phantom: Default::default(),
        }
    }
    /// Registers a callback closure on an activity with a specific topic to listen to.
    ///
    /// By default, the activity will only receive calls when it is active.
    /// Use `subscribe_masked` for more control over this behavior.
    pub fn subscribe<F>(&self, topic: Topic, f: F)
    where
        F: Fn(&mut A) + 'static,
    {
        crate::nut::register(*self, topic, move |activity, _| f(activity), Default::default())
    }

    /// Registers a callback closure on an activity with a specific topic to listen to.
    pub fn subscribe_masked<F>(&self, topic: Topic, mask: SubscriptionFilter, f: F)
    where
        F: Fn(&mut A) + 'static,
    {
        crate::nut::register(*self, topic, move |activity, _| f(activity), mask)
    }
}

impl ActivityContainer {
    pub(crate) fn add<A: Activity>(&mut self, a: A, domain: DomainId, start_active: bool) -> ActivityId<A> {
        let i = self.data.len();
        self.data.push(Some(Box::new(a)));
        self.active.push(start_active);
        ActivityId::new(i, domain)
    }
    pub(crate) fn is_active<A: Activity>(&self, id: ActivityId<A>) -> bool {
        self.active[id.index]
    }
    pub(crate) fn set_active<A: Activity>(&mut self, id: ActivityId<A>, active: bool) {
        self.active[id.index] = active
    }
}

impl<A: Activity> Index<ActivityId<A>> for ActivityContainer {
    type Output = dyn Any;
    fn index(&self, id: ActivityId<A>) -> &Self::Output {
        self.data[id.index]
            .as_ref()
            .expect("Missing activity")
            .as_ref()
    }
}
impl<A: Activity> IndexMut<ActivityId<A>> for ActivityContainer {
    fn index_mut(&mut self, id: ActivityId<A>) -> &mut Self::Output {
        self.data[id.index]
            .as_mut()
            .expect("Missing activity")
            .as_mut()
    }
}

impl ActivityHandlerContainer {
    pub fn iter_for<A: Activity>(&self, id: ActivityId<A>) -> impl Iterator<Item = &Handler> {
        self.data.get(&id.index).into_iter().flat_map(|f| f.iter())
    }
}
impl<A: Activity> Index<ActivityId<A>> for ActivityHandlerContainer {
    type Output = Vec<Handler>;
    fn index(&self, id: ActivityId<A>) -> &Self::Output {
        &self.data[&id.index]
    }
}
impl<A: Activity> IndexMut<ActivityId<A>> for ActivityHandlerContainer {
    fn index_mut(&mut self, id: ActivityId<A>) -> &mut Self::Output {
        self.data.entry(id.index).or_insert(Default::default())
    }
}

impl<A> Copy for ActivityId<A> {}
impl<A> Clone for ActivityId<A> {
    fn clone(&self) -> Self {
        *self
    }
}
