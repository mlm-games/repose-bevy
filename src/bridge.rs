use std::sync::Arc;

use bevy::prelude::*;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use repose_core::Modifier;

use crate::state::BevyClickQueue;

#[derive(Resource, Clone, Debug)]
pub struct ReposeShared<T>(Arc<RwLock<T>>);

impl<T> ReposeShared<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(value: T) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.0.read()
    }
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.0.write()
    }
    pub fn set(&self, value: T) {
        *self.0.write() = value;
    }
    pub fn get_cloned(&self) -> T
    where
        T: Clone,
    {
        self.0.read().clone()
    }
    pub fn arc(&self) -> Arc<RwLock<T>> {
        self.0.clone()
    }
}

impl<T> Default for ReposeShared<T>
where
    T: Default + Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[derive(Resource, Clone)]
pub struct ReposeChannel<T>(pub Arc<RwLock<Vec<T>>>);

impl<T> Default for ReposeChannel<T> {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(Vec::new())))
    }
}

impl<T> ReposeChannel<T> {
    pub fn send(&self, value: T) {
        self.0.write().push(value);
    }
    pub fn drain(&self) -> Vec<T> {
        std::mem::take(&mut *self.0.write())
    }
    pub fn arc(&self) -> Arc<RwLock<Vec<T>>> {
        self.0.clone()
    }
}

use std::cell::RefCell;
thread_local! {
    static CURRENT_PENDING: RefCell<Option<BevyClickQueue>> = const { RefCell::new(None) };
}

pub fn pending_scope<R>(pending: &BevyClickQueue, f: impl FnOnce() -> R) -> R {
    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            CURRENT_PENDING.with(|c| *c.borrow_mut() = None);
        }
    }
    CURRENT_PENDING.with(|c| *c.borrow_mut() = Some(Arc::clone(pending)));
    let _guard = Guard;
    let r = f();
    drop(_guard);
    r
}

pub fn bevy_click_drain_system(world: &mut World) {
    let pending = if let Some(state) = world.get_non_send::<crate::state::ReposeState>() {
        Arc::clone(&state.pending_bevy_clicks)
    } else {
        return;
    };
    let cbs: Vec<Box<dyn FnOnce(&mut World) + Send>> = std::mem::take(&mut *pending.lock());
    for cb in cbs {
        cb(world);
    }
}

pub trait BevyModifierExt {
    fn on_click_bevy(self, f: impl Fn(&mut World) + Send + Sync + 'static) -> Self;
}

impl BevyModifierExt for Modifier {
    fn on_click_bevy(self, f: impl Fn(&mut World) + Send + Sync + 'static) -> Self {
        let f = Arc::new(f);
        let queue_at_compose = CURRENT_PENDING.with(|c| c.borrow().clone());
        self.on_click(move || {
            let f_clone = Arc::clone(&f);
            let cb: Box<dyn FnOnce(&mut World) + Send> =
                Box::new(move |world| f_clone(world));
            if let Some(q) = &queue_at_compose {
                q.lock().push(cb);
            } else {
                let q2 = CURRENT_PENDING.with(|c| c.borrow().clone());
                if let Some(q2) = q2 {
                    q2.lock().push(cb);
                } else {
                    bevy::log::warn!("on_click_bevy called outside compose — callback dropped");
                }
            }
        })
    }
}
