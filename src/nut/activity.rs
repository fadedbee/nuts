use crate::nut::iac::{filter::SubscriptionFilter, managed_state::DomainId};
use crate::nut::Handler;
use crate::*;
use core::any::Any;
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
    /// Registers a callback closure that is called when an activity changes from inactive to active.
    pub fn on_enter<F>(&self, f: F)
    where
        F: Fn(&mut A) + 'static,
    {
        crate::nut::register_no_payload(*self, f, Topic::enter())
    }
    /// Same as `on_enter` but with domain access in closure
    pub fn on_enter_domained<F>(&self, f: F)
    where
        F: Fn(&mut A, &mut DomainState) + 'static,
    {
        crate::nut::register_domained_no_payload(*self, f, Topic::enter())
    }
    /// Registers a callback closure that is called when an activity changes from active to inactive.
    pub fn on_leave<F>(&self, f: F)
    where
        F: Fn(&mut A) + 'static,
    {
        crate::nut::register_no_payload(*self, f, Topic::leave())
    }
    /// Same as `on_leave` but with domain access in closure
    pub fn on_leave_domained<F>(&self, f: F)
    where
        F: Fn(&mut A, &mut DomainState) + 'static,
    {
        crate::nut::register_domained_no_payload(*self, f, Topic::leave())
    }
    /// Registers a callback closure on an activity with a specific topic to listen to.
    ///
    /// By default, the activity will only receive calls when it is active.
    /// Use `subscribe_masked` for more control over this behavior.
    pub fn subscribe<F, MSG>(&self, f: F)
    where
        F: Fn(&mut A, &MSG) + 'static,
        MSG: Any,
    {
        crate::nut::register(*self, f, Default::default())
    }
    pub fn subscribe_mut<F, MSG>(&self, f: F)
    where
        F: Fn(&mut A, &mut MSG) + 'static,
        MSG: Any,
    {
        crate::nut::register_mut(*self, f, Default::default())
    }

    /// Registers a callback closure on an activity with a specific topic to listen to.
    /// Has mutable access to the DomainState object.
    ///
    /// By default, the activity will only receive calls when it is active.
    /// Use `subscribe_domained_masked` for more control over this behavior.
    ///
    /// # Panics
    /// Panics if the activity has not been registered with a domain.    
    pub fn subscribe_domained<F, MSG>(&self, f: F)
    where
        F: Fn(&mut A, &mut DomainState, &MSG) + 'static,
        MSG: Any,
    {
        crate::nut::register_domained(*self, f, Default::default())
    }
    pub fn subscribe_domained_mut<F, MSG>(&self, f: F)
    where
        F: Fn(&mut A, &mut DomainState, &mut MSG) + 'static,
        MSG: Any,
    {
        crate::nut::register_domained_mut(*self, f, Default::default())
    }

    /// Registers a callback closure on an activity with a specific topic to listen to with filtering options.
    pub fn subscribe_masked<F, MSG>(&self, mask: SubscriptionFilter, f: F)
    where
        F: Fn(&mut A, &MSG) + 'static,
        MSG: Any,
    {
        crate::nut::register(*self, f, mask)
    }
    pub fn subscribe_masked_mut<F, MSG>(&self, mask: SubscriptionFilter, f: F)
    where
        F: Fn(&mut A, &mut MSG) + 'static,
        MSG: Any,
    {
        crate::nut::register_mut(*self, f, mask)
    }

    /// Registers a callback closure on an activity with a specific topic to listen to with filtering options.
    /// Has mutable access to the DomainState object.
    ///
    /// # Panics
    /// Panics if the activity has not been registered with a domain.
    pub fn subscribe_domained_masked<F, MSG>(&self, mask: SubscriptionFilter, f: F)
    where
        F: Fn(&mut A, &mut DomainState, &MSG) + 'static,
        MSG: Any,
    {
        crate::nut::register_domained(*self, f, mask)
    }
    pub fn subscribe_domained_masked_mut<F, MSG>(&self, mask: SubscriptionFilter, f: F)
    where
        F: Fn(&mut A, &mut DomainState, &mut MSG) + 'static,
        MSG: Any,
    {
        crate::nut::register_domained_mut(*self, f, mask)
    }
}

impl ActivityContainer {
    pub(crate) fn add<A: Activity>(
        &mut self,
        a: A,
        domain: DomainId,
        start_active: bool,
    ) -> ActivityId<A> {
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
    pub fn iter(&self) -> impl Iterator<Item = &Handler> {
        self.data.values().flat_map(|f| f.iter())
    }
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
