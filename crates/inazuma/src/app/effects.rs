use std::{
    any::{Any, TypeId},
    sync::atomic::Ordering::SeqCst,
};

use super::Effect;
#[cfg(any(test, feature = "test-support"))]
use crate::AppContext;
use crate::{AnyEntity, App, EntityId, WindowId};

impl App {
    pub(crate) fn push_effect(&mut self, effect: Effect) {
        match &effect {
            Effect::Notify { emitter } => {
                if !self.pending_notifications.insert(*emitter) {
                    return;
                }
            }
            Effect::NotifyGlobalObservers { global_type } => {
                if !self.pending_global_notifications.insert(*global_type) {
                    return;
                }
            }
            _ => {}
        };

        self.pending_effects.push_back(effect);
    }

    /// Called at the end of [`App::update`] to complete any side effects
    /// such as notifying observers, emitting events, etc. Effects can themselves
    /// cause effects, so we continue looping until all effects are processed.
    pub(super) fn flush_effects(&mut self) {
        loop {
            self.release_dropped_entities();
            self.release_dropped_focus_handles();
            if let Some(effect) = self.pending_effects.pop_front() {
                match effect {
                    Effect::Notify { emitter } => {
                        self.apply_notify_effect(emitter);
                    }

                    Effect::Emit {
                        emitter,
                        event_type,
                        event,
                    } => self.apply_emit_effect(emitter, event_type, &*event),

                    Effect::RefreshWindows => {
                        self.apply_refresh_effect();
                    }

                    Effect::NotifyGlobalObservers { global_type } => {
                        self.apply_notify_global_observers_effect(global_type);
                    }

                    Effect::Defer { callback } => {
                        self.apply_defer_effect(callback);
                    }
                    Effect::EntityCreated {
                        entity,
                        tid,
                        window,
                    } => {
                        self.apply_entity_created_effect(entity, tid, window);
                    }
                }
            } else {
                #[cfg(any(test, feature = "test-support"))]
                for window in self
                    .windows
                    .values()
                    .filter_map(|window| {
                        let window = window.as_deref()?;
                        window.invalidator.is_dirty().then_some(window.handle)
                    })
                    .collect::<Vec<_>>()
                {
                    self.update_window(window, |_, window, cx| window.draw(cx).clear())
                        .unwrap();
                }

                if self.pending_effects.is_empty() {
                    self.event_arena.clear();
                    break;
                }
            }
        }
    }

    /// Repeatedly called during `flush_effects` to release any entities whose
    /// reference count has become zero. We invoke any release observers before dropping
    /// each entity.
    fn release_dropped_entities(&mut self) {
        loop {
            let dropped = self.entities.take_dropped();
            if dropped.is_empty() {
                break;
            }

            for (entity_id, mut entity) in dropped {
                self.observers.remove(&entity_id);
                self.event_listeners.remove(&entity_id);
                for release_callback in self.release_listeners.remove(&entity_id) {
                    release_callback(entity.as_mut(), self);
                }
            }
        }
    }

    /// Repeatedly called during `flush_effects` to handle a focused handle being dropped.
    fn release_dropped_focus_handles(&mut self) {
        self.focus_handles
            .clone()
            .write()
            .retain(|handle_id, focus| {
                if focus.ref_count.load(SeqCst) == 0 {
                    for window_handle in self.windows() {
                        window_handle
                            .update(self, |_, window, _| {
                                if window.focus == Some(handle_id) {
                                    window.blur();
                                }
                            })
                            .unwrap();
                    }
                    false
                } else {
                    true
                }
            });
    }

    fn apply_notify_effect(&mut self, emitter: EntityId) {
        self.pending_notifications.remove(&emitter);

        self.observers
            .clone()
            .retain(&emitter, |handler| handler(self));
    }

    fn apply_emit_effect(&mut self, emitter: EntityId, event_type: TypeId, event: &dyn Any) {
        self.event_listeners
            .clone()
            .retain(&emitter, |(stored_type, handler)| {
                if *stored_type == event_type {
                    handler(event, self)
                } else {
                    true
                }
            });
    }

    fn apply_refresh_effect(&mut self) {
        for window in self.windows.values_mut() {
            if let Some(window) = window.as_deref_mut() {
                window.refreshing = true;
                window.invalidator.set_dirty(true);
            }
        }
    }

    fn apply_notify_global_observers_effect(&mut self, type_id: TypeId) {
        self.pending_global_notifications.remove(&type_id);
        self.global_observers
            .clone()
            .retain(&type_id, |observer| observer(self));
    }

    fn apply_defer_effect(&mut self, callback: Box<dyn FnOnce(&mut Self) + 'static>) {
        callback(self);
    }

    fn apply_entity_created_effect(
        &mut self,
        entity: AnyEntity,
        tid: TypeId,
        window: Option<WindowId>,
    ) {
        self.new_entity_observers.clone().retain(&tid, |observer| {
            if let Some(id) = window {
                self.update_window_id(id, {
                    let entity = entity.clone();
                    |_, window, cx| (observer)(entity, &mut Some(window), cx)
                })
                .expect("All windows should be off the stack when flushing effects");
            } else {
                (observer)(entity.clone(), &mut None, self)
            }
            true
        });
    }
}
