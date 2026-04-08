use super::*;

/// A type erased, weak reference to a entity.
#[derive(Clone)]
pub struct AnyWeakEntity {
    pub(crate) entity_id: EntityId,
    pub(super) entity_type: TypeId,
    pub(super) entity_ref_counts: Weak<RwLock<EntityRefCounts>>,
}

impl AnyWeakEntity {
    /// Get the entity ID associated with this weak reference.
    #[inline]
    pub fn entity_id(&self) -> EntityId {
        self.entity_id
    }

    /// Check if this weak handle can be upgraded, or if the entity has already been dropped
    pub fn is_upgradable(&self) -> bool {
        let ref_count = self
            .entity_ref_counts
            .upgrade()
            .and_then(|ref_counts| Some(ref_counts.read().counts.get(self.entity_id)?.load(SeqCst)))
            .unwrap_or(0);
        ref_count > 0
    }

    /// Upgrade this weak entity reference to a strong reference.
    pub fn upgrade(&self) -> Option<AnyEntity> {
        let ref_counts = &self.entity_ref_counts.upgrade()?;
        let ref_counts = ref_counts.read();
        let ref_count = ref_counts.counts.get(self.entity_id)?;

        if atomic_incr_if_not_zero(ref_count) == 0 {
            // entity_id is in dropped_entity_ids
            return None;
        }
        drop(ref_counts);

        Some(AnyEntity {
            entity_id: self.entity_id,
            entity_type: self.entity_type,
            entity_map: self.entity_ref_counts.clone(),
            #[cfg(any(test, feature = "leak-detection"))]
            handle_id: self
                .entity_ref_counts
                .upgrade()
                .unwrap()
                .write()
                .leak_detector
                .handle_created(self.entity_id, None),
        })
    }

    /// Asserts that the entity referenced by this weak handle has been fully released.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let entity = cx.new(|_| MyEntity::new());
    /// let weak = entity.downgrade();
    /// drop(entity);
    ///
    /// // Verify the entity was released
    /// weak.assert_released();
    /// ```
    ///
    /// # Debugging Leaks
    ///
    /// If this method panics due to leaked handles, set the `LEAK_BACKTRACE` environment
    /// variable to see where the leaked handles were allocated:
    ///
    /// ```bash
    /// LEAK_BACKTRACE=1 cargo test my_test
    /// ```
    ///
    /// # Panics
    ///
    /// - Panics if any strong handles to the entity are still alive.
    /// - Panics if the entity was recently dropped but cleanup hasn't completed yet
    ///   (resources are retained until the end of the effect cycle).
    #[cfg(any(test, feature = "leak-detection"))]
    pub fn assert_released(&self) {
        self.entity_ref_counts
            .upgrade()
            .unwrap()
            .write()
            .leak_detector
            .assert_released(self.entity_id);

        if self
            .entity_ref_counts
            .upgrade()
            .and_then(|ref_counts| Some(ref_counts.read().counts.get(self.entity_id)?.load(SeqCst)))
            .is_some()
        {
            panic!(
                "entity was recently dropped but resources are retained until the end of the effect cycle."
            )
        }
    }

    /// Creates a weak entity that can never be upgraded.
    pub fn new_invalid() -> Self {
        /// To hold the invariant that all ids are unique, and considering that slotmap
        /// increases their IDs from `0`, we can decrease ours from `u64::MAX` so these
        /// two will never conflict (u64 is way too large).
        static UNIQUE_NON_CONFLICTING_ID_GENERATOR: AtomicU64 = AtomicU64::new(u64::MAX);
        let entity_id = UNIQUE_NON_CONFLICTING_ID_GENERATOR.fetch_sub(1, SeqCst);

        Self {
            // Safety:
            //   Docs say this is safe but can be unspecified if slotmap changes the representation
            //   after `1.0.7`, that said, providing a valid entity_id here is not necessary as long
            //   as we guarantee that `entity_id` is never used if `entity_ref_counts` equals
            //   to `Weak::new()` (that is, it's unable to upgrade), that is the invariant that
            //   actually needs to be hold true.
            //
            //   And there is no sane reason to read an entity slot if `entity_ref_counts` can't be
            //   read in the first place, so we're good!
            entity_id: entity_id.into(),
            entity_type: TypeId::of::<()>(),
            entity_ref_counts: Weak::new(),
        }
    }
}

impl std::fmt::Debug for AnyWeakEntity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(type_name::<Self>())
            .field("entity_id", &self.entity_id)
            .field("entity_type", &self.entity_type)
            .finish()
    }
}

impl<T> From<WeakEntity<T>> for AnyWeakEntity {
    #[inline]
    fn from(entity: WeakEntity<T>) -> Self {
        entity.any_entity
    }
}

impl Hash for AnyWeakEntity {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity_id.hash(state);
    }
}

impl PartialEq for AnyWeakEntity {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.entity_id == other.entity_id
    }
}

impl Eq for AnyWeakEntity {}

impl Ord for AnyWeakEntity {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.entity_id.cmp(&other.entity_id)
    }
}

impl PartialOrd for AnyWeakEntity {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A weak reference to a entity of the given type.
#[derive(Deref, DerefMut)]
pub struct WeakEntity<T> {
    #[deref]
    #[deref_mut]
    pub(super) any_entity: AnyWeakEntity,
    pub(crate) entity_type: PhantomData<fn(T) -> T>,
}

impl<T> std::fmt::Debug for WeakEntity<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(type_name::<Self>())
            .field("entity_id", &self.any_entity.entity_id)
            .field("entity_type", &type_name::<T>())
            .finish()
    }
}

impl<T> Clone for WeakEntity<T> {
    fn clone(&self) -> Self {
        Self {
            any_entity: self.any_entity.clone(),
            entity_type: self.entity_type,
        }
    }
}

impl<T: 'static> WeakEntity<T> {
    /// Upgrade this weak entity reference into a strong entity reference
    pub fn upgrade(&self) -> Option<Entity<T>> {
        Some(Entity {
            any_entity: self.any_entity.upgrade()?,
            entity_type: self.entity_type,
        })
    }

    /// Updates the entity referenced by this handle with the given function if
    /// the referenced entity still exists. Returns an error if the entity has
    /// been released.
    pub fn update<C, R>(
        &self,
        cx: &mut C,
        update: impl FnOnce(&mut T, &mut Context<T>) -> R,
    ) -> Result<R>
    where
        C: AppContext,
    {
        let entity = self.upgrade().context("entity released")?;
        Ok(cx.update_entity(&entity, update))
    }

    /// Updates the entity referenced by this handle with the given function if
    /// the referenced entity still exists, within a visual context that has a window.
    /// Returns an error if the entity has been released.
    pub fn update_in<C, R>(
        &self,
        cx: &mut C,
        update: impl FnOnce(&mut T, &mut Window, &mut Context<T>) -> R,
    ) -> Result<R>
    where
        C: VisualContext,
    {
        let window = cx.window_handle();
        let entity = self.upgrade().context("entity released")?;

        window.update(cx, |_, window, cx| {
            entity.update(cx, |entity, cx| update(entity, window, cx))
        })
    }

    /// Reads the entity referenced by this handle with the given function if
    /// the referenced entity still exists. Returns an error if the entity has
    /// been released.
    pub fn read_with<C, R>(&self, cx: &C, read: impl FnOnce(&T, &App) -> R) -> Result<R>
    where
        C: AppContext,
    {
        let entity = self.upgrade().context("entity released")?;
        Ok(cx.read_entity(&entity, read))
    }

    /// Create a new weak entity that can never be upgraded.
    #[inline]
    pub fn new_invalid() -> Self {
        Self {
            any_entity: AnyWeakEntity::new_invalid(),
            entity_type: PhantomData,
        }
    }
}

impl<T> Hash for WeakEntity<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.any_entity.hash(state);
    }
}

impl<T> PartialEq for WeakEntity<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.any_entity == other.any_entity
    }
}

impl<T> Eq for WeakEntity<T> {}

impl<T> PartialEq<Entity<T>> for WeakEntity<T> {
    #[inline]
    fn eq(&self, other: &Entity<T>) -> bool {
        self.entity_id() == other.any_entity.entity_id()
    }
}

impl<T: 'static> Ord for WeakEntity<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.entity_id().cmp(&other.entity_id())
    }
}

impl<T: 'static> PartialOrd for WeakEntity<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Controls whether backtraces are captured when entity handles are created.
///
/// Set the `LEAK_BACKTRACE` environment variable to any non-empty value to enable
/// backtrace capture. This helps identify where leaked handles were allocated.
#[cfg(any(test, feature = "leak-detection"))]
static LEAK_BACKTRACE: std::sync::LazyLock<bool> =
    std::sync::LazyLock::new(|| std::env::var("LEAK_BACKTRACE").is_ok_and(|b| !b.is_empty()));

/// Unique identifier for a specific entity handle instance.
///
/// This is distinct from `EntityId` - while multiple handles can point to the same
/// entity (same `EntityId`), each handle has its own unique `HandleId`.
#[cfg(any(test, feature = "leak-detection"))]
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub(crate) struct HandleId {
    id: u64,
}

/// Tracks entity handle allocations to detect leaks.
///
/// The leak detector is enabled in tests and when the `leak-detection` feature is active.
/// It tracks every `Entity<T>` and `AnyEntity` handle that is created and released,
/// allowing you to verify that all handles to an entity have been properly dropped.
///
/// # How do leaks happen?
///
/// Entities are reference-counted structures that can own other entities
/// allowing to form cycles. If such a strong-reference counted cycle is
/// created, all participating strong entities in this cycle will effectively
/// leak as they cannot be released anymore.
///
/// # Usage
///
/// You can use `WeakEntity::assert_released` or `AnyWeakEntity::assert_released`
/// to verify that an entity has been fully released:
///
/// ```ignore
/// let entity = cx.new(|_| MyEntity::new());
/// let weak = entity.downgrade();
/// drop(entity);
///
/// // This will panic if any handles to the entity are still alive
/// weak.assert_released();
/// ```
///
/// # Debugging Leaks
///
/// When a leak is detected, the detector will panic with information about the leaked
/// handles. To see where the leaked handles were allocated, set the `LEAK_BACKTRACE`
/// environment variable:
///
/// ```bash
/// LEAK_BACKTRACE=1 cargo test my_test
/// ```
///
/// This will capture and display backtraces for each leaked handle, helping you
/// identify where handles were created but not released.
///
/// # How It Works
///
/// - When an entity handle is created (via `Entity::new`, `Entity::clone`, or
///   `WeakEntity::upgrade`), `handle_created` is called to register the handle.
/// - When a handle is dropped, `handle_released` removes it from tracking.
/// - `assert_released` verifies that no handles remain for a given entity.
#[cfg(any(test, feature = "leak-detection"))]
pub(crate) struct LeakDetector {
    pub(super) next_handle_id: u64,
    pub(super) entity_handles: HashMap<EntityId, EntityLeakData>,
}

/// A snapshot of the set of alive entities at a point in time.
///
/// Created by [`LeakDetector::snapshot`]. Can later be passed to
/// [`LeakDetector::assert_no_new_leaks`] to verify that no new entity
/// handles remain between the snapshot and the current state.
#[cfg(any(test, feature = "leak-detection"))]
pub struct LeakDetectorSnapshot {
    entity_ids: inazuma_collections::HashSet<EntityId>,
}

#[cfg(any(test, feature = "leak-detection"))]
pub(super) struct EntityLeakData {
    handles: HashMap<HandleId, Option<backtrace::Backtrace>>,
    type_name: &'static str,
}

#[cfg(any(test, feature = "leak-detection"))]
impl LeakDetector {
    /// Records that a new handle has been created for the given entity.
    ///
    /// Returns a unique `HandleId` that must be passed to `handle_released` when
    /// the handle is dropped. If `LEAK_BACKTRACE` is set, captures a backtrace
    /// at the allocation site.
    #[track_caller]
    pub fn handle_created(
        &mut self,
        entity_id: EntityId,
        type_name: Option<&'static str>,
    ) -> HandleId {
        let id = inazuma_util::post_inc(&mut self.next_handle_id);
        let handle_id = HandleId { id };
        let handles = self
            .entity_handles
            .entry(entity_id)
            .or_insert_with(|| EntityLeakData {
                handles: HashMap::default(),
                type_name: type_name.unwrap_or("<unknown>"),
            });
        handles.handles.insert(
            handle_id,
            LEAK_BACKTRACE.then(backtrace::Backtrace::new_unresolved),
        );
        handle_id
    }

    /// Records that a handle has been released (dropped).
    ///
    /// This removes the handle from tracking. The `handle_id` should be the same
    /// one returned by `handle_created` when the handle was allocated.
    pub fn handle_released(&mut self, entity_id: EntityId, handle_id: HandleId) {
        if let std::collections::hash_map::Entry::Occupied(mut data) =
            self.entity_handles.entry(entity_id)
        {
            data.get_mut().handles.remove(&handle_id);
            if data.get().handles.is_empty() {
                data.remove();
            }
        }
    }

    /// Asserts that all handles to the given entity have been released.
    ///
    /// # Panics
    ///
    /// Panics if any handles to the entity are still alive. The panic message
    /// includes backtraces for each leaked handle if `LEAK_BACKTRACE` is set,
    /// otherwise it suggests setting the environment variable to get more info.
    pub fn assert_released(&mut self, entity_id: EntityId) {
        use std::fmt::Write as _;
        if let Some(data) = self.entity_handles.remove(&entity_id) {
            let mut out = String::new();
            for (_, backtrace) in data.handles {
                if let Some(mut backtrace) = backtrace {
                    backtrace.resolve();
                    writeln!(out, "Leaked handle:\n{:?}", backtrace).unwrap();
                } else {
                    writeln!(
                        out,
                        "Leaked handle: (export LEAK_BACKTRACE to find allocation site)"
                    )
                    .unwrap();
                }
            }
            panic!("{out}");
        }
    }

    /// Captures a snapshot of all entity IDs that currently have alive handles.
    ///
    /// The returned [`LeakDetectorSnapshot`] can later be passed to
    /// [`assert_no_new_leaks`](Self::assert_no_new_leaks) to verify that no
    /// entities created after the snapshot are still alive.
    pub fn snapshot(&self) -> LeakDetectorSnapshot {
        LeakDetectorSnapshot {
            entity_ids: self.entity_handles.keys().copied().collect(),
        }
    }

    /// Asserts that no entities created after `snapshot` still have alive handles.
    ///
    /// Entities that were already tracked at the time of the snapshot are ignored,
    /// even if they still have handles. Only *new* entities (those whose
    /// `EntityId` was not present in the snapshot) are considered leaks.
    ///
    /// # Panics
    ///
    /// Panics if any new entity handles exist. The panic message lists every
    /// leaked entity with its type name, and includes allocation-site backtraces
    /// when `LEAK_BACKTRACE` is set.
    pub fn assert_no_new_leaks(&self, snapshot: &LeakDetectorSnapshot) {
        use std::fmt::Write as _;

        let mut out = String::new();
        for (entity_id, data) in &self.entity_handles {
            if snapshot.entity_ids.contains(entity_id) {
                continue;
            }
            for (_, backtrace) in &data.handles {
                if let Some(backtrace) = backtrace {
                    let mut backtrace = backtrace.clone();
                    backtrace.resolve();
                    writeln!(
                        out,
                        "Leaked handle for entity {} ({entity_id:?}):\n{:?}",
                        data.type_name, backtrace
                    )
                    .unwrap();
                } else {
                    writeln!(
                        out,
                        "Leaked handle for entity {} ({entity_id:?}): (export LEAK_BACKTRACE to find allocation site)",
                        data.type_name
                    )
                    .unwrap();
                }
            }
        }

        if !out.is_empty() {
            panic!("New entity leaks detected since snapshot:\n{out}");
        }
    }
}

#[cfg(any(test, feature = "leak-detection"))]
impl Drop for LeakDetector {
    fn drop(&mut self) {
        use std::fmt::Write;

        if self.entity_handles.is_empty() || std::thread::panicking() {
            return;
        }

        let mut out = String::new();
        for (entity_id, data) in self.entity_handles.drain() {
            for (_handle, backtrace) in data.handles {
                if let Some(mut backtrace) = backtrace {
                    backtrace.resolve();
                    writeln!(
                        out,
                        "Leaked handle for entity {} ({entity_id:?}):\n{:?}",
                        data.type_name, backtrace
                    )
                    .unwrap();
                } else {
                    writeln!(
                        out,
                        "Leaked handle for entity {} ({entity_id:?}): (export LEAK_BACKTRACE to find allocation site)",
                        data.type_name
                    )
                    .unwrap();
                }
            }
        }
        panic!("Exited with leaked handles:\n{out}");
    }
}

#[cfg(test)]
mod test {
    use crate::EntityMap;

    struct TestEntity {
        pub i: i32,
    }

    #[test]
    fn test_entity_map_slot_assignment_before_cleanup() {
        // Tests that slots are not re-used before take_dropped.
        let mut entity_map = EntityMap::new();

        let slot = entity_map.reserve::<TestEntity>();
        entity_map.insert(slot, TestEntity { i: 1 });

        let slot = entity_map.reserve::<TestEntity>();
        entity_map.insert(slot, TestEntity { i: 2 });

        let dropped = entity_map.take_dropped();
        assert_eq!(dropped.len(), 2);

        assert_eq!(
            dropped
                .into_iter()
                .map(|(_, entity)| entity.downcast::<TestEntity>().unwrap().i)
                .collect::<Vec<i32>>(),
            vec![1, 2],
        );
    }

    #[test]
    fn test_entity_map_weak_upgrade_before_cleanup() {
        // Tests that weak handles are not upgraded before take_dropped
        let mut entity_map = EntityMap::new();

        let slot = entity_map.reserve::<TestEntity>();
        let handle = entity_map.insert(slot, TestEntity { i: 1 });
        let weak = handle.downgrade();
        drop(handle);

        let strong = weak.upgrade();
        assert_eq!(strong, None);

        let dropped = entity_map.take_dropped();
        assert_eq!(dropped.len(), 1);

        assert_eq!(
            dropped
                .into_iter()
                .map(|(_, entity)| entity.downcast::<TestEntity>().unwrap().i)
                .collect::<Vec<i32>>(),
            vec![1],
        );
    }

    #[test]
    fn test_leak_detector_snapshot_no_leaks() {
        let mut entity_map = EntityMap::new();

        let slot = entity_map.reserve::<TestEntity>();
        let pre_existing = entity_map.insert(slot, TestEntity { i: 1 });

        let snapshot = entity_map.leak_detector_snapshot();

        let slot = entity_map.reserve::<TestEntity>();
        let temporary = entity_map.insert(slot, TestEntity { i: 2 });
        drop(temporary);

        entity_map.assert_no_new_leaks(&snapshot);

        drop(pre_existing);
    }

    #[test]
    #[should_panic(expected = "New entity leaks detected since snapshot")]
    fn test_leak_detector_snapshot_detects_new_leak() {
        let mut entity_map = EntityMap::new();

        let slot = entity_map.reserve::<TestEntity>();
        let pre_existing = entity_map.insert(slot, TestEntity { i: 1 });

        let snapshot = entity_map.leak_detector_snapshot();

        let slot = entity_map.reserve::<TestEntity>();
        let leaked = entity_map.insert(slot, TestEntity { i: 2 });

        // `leaked` is still alive, so this should panic.
        entity_map.assert_no_new_leaks(&snapshot);

        drop(pre_existing);
        drop(leaked);
    }
}
